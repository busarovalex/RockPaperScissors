extern crate rand;
extern crate rustc_serialize;

/// Width of game board
#[cfg(not(test))] pub const WIDTH: usize = 8;
#[cfg(test)]      pub const WIDTH: usize = 3;
/// Height of game board
#[cfg(not(test))] pub const HEIGHT: usize = 8;
#[cfg(test)]      pub const HEIGHT: usize = 3;
/// Number of rows
#[cfg(not(test))] pub const ROWS: usize = 2;
#[cfg(test)]      pub const ROWS: usize = 1;

const RED: Player = Player::Red;
const BLUE: Player = Player::Blue;

const ROCK: RPS = RPS::Rock;
const PAPER: RPS = RPS::Paper;
const SCISSORS: RPS = RPS::Scissors;

const WIN: Outcome = Outcome::Win;
const LOSE: Outcome = Outcome::Lose;
const DRAW: Outcome = Outcome::Draw;

pub mod move_conditions;
pub mod win_conditions;
pub mod unit;
pub mod field;

use move_conditions::{MoveCondition, Move};
use win_conditions::{WinCondition};
use field::{Field, PovField};
use unit::{Unit, GeneralUnit};

use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct Game<T: MoveCondition, E: WinCondition<GeneralUnit>> {
    turns: u32,
    current_turn: Player,
    winner: Option<Player>,
    field: Field<GeneralUnit>,
    rules: Rules<GeneralUnit, T, E>,
}

impl<T: MoveCondition, E: WinCondition<GeneralUnit>> Game<T, E> {
    pub fn new(rules: Rules<GeneralUnit, T, E>) -> Game<T, E> {
        let mut rows = [[None; WIDTH]; HEIGHT];
        for i in 0..ROWS {
            rows[i] = [Some(RED.random_unit()); WIDTH];
            rows[HEIGHT - i - 1] = [Some(BLUE.random_unit()); WIDTH];
        }
        let field = Field { rows: rows };
        Game {
            turns: 1,
            current_turn: RED,
            winner: None,
            field: field,
            rules: rules,
        }
    }
    
    pub fn turns(&self) -> u32 { self.turns }
    pub fn current_turn(&self) -> Player { self.current_turn }
    pub fn winner(&self) -> Option<Player> { self.winner }
    pub fn field(&self) -> &Field<GeneralUnit> { &self.field }
    
    pub fn force_win(&mut self, player: Player) {
        if !self.winner.is_some() { self.winner = Some(player); }
    }
    
    pub fn perspective(&self, player: Player) -> PovField {
        PovField::from((&self.field, player))
    }
    
    pub fn make_move(&mut self, movement: Move) -> Result<Option<Outcome>, MoveError> {
        if self.winner.is_some() { return Err(MoveError::GameAlreadyFinished); }
        
        if !self.rules.move_condition.is_valid(movement) {
            return Err(MoveError::DeclinedByMoveCondition);
        }
        
        let (from_x, from_y) = movement.from;
        
        if from_x >= WIDTH || from_y >= HEIGHT { return Err(MoveError::PositionOutOfBounds); }
        
        let attack_outcome;
        let (to_x, to_y);
        
        if let Some(ref unit) = self.field.rows[from_y][from_x].as_ref() {
            if unit.owner != self.current_turn { return Err(MoveError::WrongOwner); }
            let dist = movement.apply(unit.owner);
            to_x = dist.0;
            to_y = dist.1; 
            if to_x >= WIDTH || to_y >= HEIGHT { return Err(MoveError::PositionOutOfBounds); }
            
            if let Some(ref defender) = self.field.rows[to_y][to_x].as_ref() {
                if defender.owner == self.current_turn { return Err(MoveError::SameOwner); }
                
                match unit.attack(defender) {
                    Some(res) => {
                        attack_outcome = Some(res);
                    },
                    None => { return Err(MoveError::UnexpextedError); }
                } 
            } else {
                attack_outcome = None;
            }
            
            
            
        } else {
            return Err(MoveError::NoUnitInPosition);
        }
        
        if let Some(outcome) = attack_outcome {
            match outcome {
                WIN => {
                    self.field.rows[to_y][to_x] = self.field.rows[from_y][from_x];
                    self.field.rows[from_y][from_x] = None;
                    self.field.rows[to_y][to_x].as_mut().unwrap().visible = true;
                },
                LOSE => {
                    self.field.rows[from_y][from_x] = None;
                    self.field.rows[to_y][to_x].as_mut().unwrap().visible = true;
                },
                DRAW => {
                    self.field.rows[from_y][from_x].as_mut().unwrap().visible = true;
                    self.field.rows[to_y][to_x].as_mut().unwrap().visible = true;
                }
            }
        } else {
            self.field.rows[to_y][to_x] = self.field.rows[from_y][from_x];
            self.field.rows[from_y][from_x] = None;
        }
        
        self.winner = self.rules.win_condition.winner(&self.field);
        self.turns += 1;
        self.current_turn = self.current_turn.next();
        
        Ok(attack_outcome)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveError {
    GameAlreadyFinished,
    DeclinedByMoveCondition,
    PositionOutOfBounds,
    WrongOwner,
    NoUnitInPosition,
    SameOwner,
    UnexpextedError,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, RustcEncodable, RustcDecodable)]
pub enum Player {
    Red,
    Blue,
}

impl Player {
    fn next(&self) -> Player {
        match *self {
            RED => BLUE,
            BLUE => RED,
        }
    }
    
    fn unit(&self, rps: RPS) -> GeneralUnit {
        GeneralUnit::new(rps, *self)
    }
    
    fn random_unit(&self) -> GeneralUnit {
        self.unit(RPS::random())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, RustcEncodable, RustcDecodable)]
pub enum RPS {
    Rock,
    Paper,
    Scissors,
}

impl RPS {
    fn attack(&self, opponent: RPS) -> Outcome {
        match (*self, opponent) {
            (PAPER, ROCK) | (ROCK, SCISSORS) | (SCISSORS, PAPER) => WIN,
            (ROCK, PAPER) | (SCISSORS, ROCK) | (PAPER, SCISSORS) => LOSE,
            _ => DRAW,
        }
    }
    
    fn random() -> RPS {
        match rand::random::<usize>() % 3 {
            0 => ROCK,
            1 => PAPER,
            2 => SCISSORS,
            _ => { panic!("rand::random::<usize>() % 3 returned not 0, nor 1, nor 2"); }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Win,
    Lose,
    Draw,
}

#[derive(Clone, Debug)]
pub struct Rules<K, T: MoveCondition, E: WinCondition<K>> where K: Unit {
    pub move_condition: T,
    pub win_condition: E,
    phantom_data: PhantomData<K>,
}

impl<K: Unit, T: MoveCondition, E: WinCondition<K>> Rules<K, T, E> {
    pub fn new(move_condition: T, win_condition: E) -> Rules<K, T, E> {
        Rules {
            move_condition: move_condition,
            win_condition: win_condition,
            phantom_data: PhantomData,
        }
    }
}

#[test]
fn basic_test() {
    use move_conditions::{OnlyForwardMove, Direction};
    use win_conditions::EliminateCondition;
    
    let rules = Rules::new(OnlyForwardMove, EliminateCondition);
    let mut game = Game::new(rules);
    let move1 = Move::new(0, 0, Direction::Forward);
    
    assert_eq!(game.make_move(move1), Ok(None));
    
    assert_eq!(game.make_move(move1), Err(MoveError::NoUnitInPosition));
    
    let move2 = Move::new(0, 2, Direction::Forward);
    assert!(game.make_move(move2).unwrap().is_some());
}
