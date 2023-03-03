use rand::{
    distributions::{Uniform, WeightedIndex},
    prelude::Distribution,
    Rng,
};
use std::collections::HashMap;

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Graph {
    pub name: String,
    pub vertices: Vec<(f32, f32)>,
    pub adjacency_list: Vec<Vec<usize>>,
}

pub fn template_graphs() -> Vec<Graph> {
    vec![
        Graph {
            name: "Path2".to_string(),
            vertices: vec![(0.5, 0.2), (0.5, 0.8)],
            adjacency_list: vec![vec![1], vec![0]],
        },
        Graph {
            name: "Path5".to_string(),
            vertices: vec![(0.5, 0.1), (0.5, 0.3), (0.5, 0.5), (0.5, 0.7), (0.5, 0.9)],
            adjacency_list: vec![vec![1], vec![0, 2], vec![1, 3], vec![2, 4], vec![3]],
        },
        Graph {
            name: "Hexagon".to_string(),
            vertices: vec![
                (0.5, 0.2),
                (0.76, 0.35),
                (0.76, 0.65),
                (0.5, 0.8),
                (0.24, 0.65),
                (0.24, 0.35),
            ],
            adjacency_list: vec![
                vec![5, 1],
                vec![0, 2],
                vec![1, 3],
                vec![2, 4],
                vec![3, 5],
                vec![4, 0],
            ],
        },
    ]
}

#[derive(Debug, PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Algorithm {
    Random,
    Menace,
}

type CopPositions = Vec<usize>;
type RobberPosition = usize;

pub trait Cop {
    fn start(&mut self, graph: &Graph) -> CopPositions;
    fn step(
        &mut self,
        graph: &Graph,
        cop_positions: &CopPositions,
        robber_position: RobberPosition,
    ) -> CopPositions;
    fn end(&mut self, graph: &Graph, cop_positions: &CopPositions, robber_position: RobberPosition);
}

pub trait Robber {
    fn start(&mut self, graph: &Graph, cop_positions: &CopPositions) -> RobberPosition;
    fn step(
        &mut self,
        graph: &Graph,
        cop_positions: &CopPositions,
        robber_position: RobberPosition,
    ) -> RobberPosition;
    fn end(&mut self, graph: &Graph, cop_positions: &CopPositions, robber_position: RobberPosition);
}

struct RandomCop {
    number_of_cops: u8,
}

impl RandomCop {
    fn new(number_of_cops: u8) -> RandomCop {
        RandomCop { number_of_cops }
    }
}

impl Cop for RandomCop {
    fn start(&mut self, graph: &Graph) -> CopPositions {
        let mut positions = Vec::new();
        let mut rng = rand::thread_rng();
        let options = Uniform::from(0..graph.vertices.len());
        for _ in 0..self.number_of_cops {
            positions.push(options.sample(&mut rng));
        }
        positions
    }

    fn step(
        &mut self,
        graph: &Graph,
        cop_positions: &CopPositions,
        _robber_position: RobberPosition,
    ) -> CopPositions {
        let mut positions = Vec::new();
        let mut rng = rand::thread_rng();
        for &cop_position in cop_positions {
            let neighbours = &graph.adjacency_list[cop_position];
            // Since we can stay at our current position, we choose a number from 0 to neighbours.len().
            // If it's neighbours.len() we stay at our position, else we move to the corresponding neighbour.
            let new_position = rng.gen_range(0..=neighbours.len());
            if new_position == neighbours.len() {
                positions.push(cop_position);
            } else {
                positions.push(neighbours[new_position]);
            }
        }
        positions
    }

    fn end(&mut self, _graph: &Graph, _cop_positions: &CopPositions, _robber_position: usize) {}
}

struct RandomRobber {}

impl RandomRobber {
    fn new() -> RandomRobber {
        RandomRobber {}
    }
}

impl Robber for RandomRobber {
    fn start(&mut self, graph: &Graph, _cop_positions: &CopPositions) -> RobberPosition {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..graph.vertices.len())
    }

    fn step(
        &mut self,
        graph: &Graph,
        _cop_positions: &CopPositions,
        robber_position: RobberPosition,
    ) -> RobberPosition {
        let mut rng = rand::thread_rng();
        let neighbours = &graph.adjacency_list[robber_position];
        let new_position = rng.gen_range(0..=neighbours.len());
        if new_position == neighbours.len() {
            robber_position
        } else {
            neighbours[new_position]
        }
    }

    fn end(
        &mut self,
        _graph: &Graph,
        _cop_positions: &CopPositions,
        _robber_position: RobberPosition,
    ) {
    }
}

// A bag of moves for a given position. Used by the MENACE algorithm.
struct Bag {
    counts: Vec<u32>,
}

impl Bag {
    fn new(size: usize) -> Bag {
        Bag {
            counts: vec![50; size],
        }
    }

    fn choose(&self) -> usize {
        let dist = WeightedIndex::new(&self.counts).unwrap();
        let mut rng = rand::thread_rng();
        dist.sample(&mut rng)
    }

    fn increase(&mut self, value: usize) {
        self.counts[value] += 3;
    }

    fn decrease(&mut self, value: usize) {
        if self.counts[value] > 0 {
            self.counts[value] -= 1;
        }

        let total_count: u32 = self.counts.iter().sum();
        if total_count == 0 {
            for count in self.counts.iter_mut() {
                *count = 50;
            }
        }
    }
}

struct MenaceCop {
    number_of_cops: u8,
    // We use Option<(CopPositions, RobberPosition)>:
    // None is the key for the bag corresponding to the start state.
    // Some((cop_positions, robber_position)) corresponds to the non start states.
    bags: HashMap<Option<(CopPositions, RobberPosition)>, Bag>,
    // We keep track of the moves to increase/decrease.
    moves: Vec<(Option<(CopPositions, RobberPosition)>, usize)>,
}

impl MenaceCop {
    fn new(number_of_cops: u8) -> Self {
        Self {
            number_of_cops,
            bags: HashMap::new(),
            moves: Vec::new(),
        }
    }
}

impl Cop for MenaceCop {
    fn start(&mut self, graph: &Graph) -> CopPositions {
        let number_of_vertices = graph.vertices.len();
        let bag_key = None;
        let bag = self
            .bags
            .entry(bag_key.clone())
            .or_insert_with(|| Bag::new(number_of_vertices.pow(self.number_of_cops as u32)));

        let mut choice = bag.choose();
        self.moves.push((bag_key, choice));

        let mut position = vec![];
        for _ in 0..self.number_of_cops {
            position.push(choice % number_of_vertices);
            choice /= number_of_vertices;
        }
        position
    }

    fn step(
        &mut self,
        graph: &Graph,
        cop_positions: &CopPositions,
        robber_position: RobberPosition,
    ) -> CopPositions {
        let bag_key = Some((cop_positions.clone(), robber_position));
        let bag = self.bags.entry(bag_key.clone()).or_insert_with(|| {
            let mut size = 1;
            for &cop_position in cop_positions {
                size *= graph.adjacency_list[cop_position].len() + 1;
            }
            Bag::new(size)
        });

        let mut choice = bag.choose();
        self.moves.push((bag_key, choice));

        let mut position = vec![];
        for &cop_position in cop_positions {
            let neighbours = &graph.adjacency_list[cop_position];
            let new_cop_position = choice % (neighbours.len() + 1);
            if new_cop_position == neighbours.len() {
                position.push(cop_position);
            } else {
                position.push(neighbours[new_cop_position]);
            }
            choice /= neighbours.len() + 1;
        }
        position
    }

    fn end(
        &mut self,
        _graph: &Graph,
        cop_positions: &CopPositions,
        robber_position: RobberPosition,
    ) {
        let won = cop_positions.contains(&robber_position);
        for (position, choice) in self.moves.iter() {
            // We should've added a corresponding bag if the position is in self.moves, so we can unwrap.
            let bag = self.bags.get_mut(position).unwrap();
            if won {
                bag.increase(*choice);
            } else {
                bag.decrease(*choice);
            }
        }
        self.moves.clear();
    }
}

struct MenaceRobber {
    // We use (CopPositions, Option<RobberPosition>):
    // (cop_positions, None) is the key for the bag corresponding to the start states.
    // (cop_positions, Some(robber_position)) corresponds to the non start states.
    bags: HashMap<(CopPositions, Option<RobberPosition>), Bag>,
    // We keep track of the moves to increase/decrease.
    moves: Vec<((CopPositions, Option<RobberPosition>), usize)>,
}

impl MenaceRobber {
    fn new() -> Self {
        Self {
            bags: HashMap::new(),
            moves: Vec::new(),
        }
    }
}

impl Robber for MenaceRobber {
    fn start(&mut self, graph: &Graph, cop_positions: &CopPositions) -> RobberPosition {
        let bag_key = (cop_positions.clone(), None);
        let bag = self.bags.entry(bag_key.clone()).or_insert_with(|| {
            let number_of_vertices = graph.vertices.len();
            Bag::new(number_of_vertices)
        });

        let new_robber_position = bag.choose();
        self.moves.push((bag_key, new_robber_position));
        new_robber_position
    }

    fn step(
        &mut self,
        graph: &Graph,
        cop_positions: &CopPositions,
        robber_position: RobberPosition,
    ) -> RobberPosition {
        let neighbours = &graph.adjacency_list[robber_position];
        let bag_key = (cop_positions.clone(), Some(robber_position));
        let bag = self
            .bags
            .entry(bag_key.clone())
            .or_insert_with(|| Bag::new(neighbours.len() + 1));

        let new_robber_position = bag.choose();
        self.moves.push((bag_key, new_robber_position));
        if new_robber_position == neighbours.len() {
            robber_position
        } else {
            neighbours[new_robber_position]
        }
    }

    fn end(
        &mut self,
        _graph: &Graph,
        cop_positions: &CopPositions,
        robber_position: RobberPosition,
    ) {
        let won = !cop_positions.contains(&robber_position);
        for (position, choice) in self.moves.iter() {
            let bag = self.bags.get_mut(position).unwrap();
            if won {
                bag.increase(*choice);
            } else {
                bag.decrease(*choice);
            }
        }
        self.moves.clear();
    }
}

#[derive(PartialEq)]
pub enum Turn {
    Cop,
    Robber,
    Over,
}

pub struct Game {
    pub graph: Graph,
    pub number_of_steps: u8,
    pub cop: Box<dyn Cop + Send>,
    pub robber: Box<dyn Robber + Send>,
    pub score: [u32; 2],
    pub cop_positions: Option<CopPositions>,
    pub robber_position: Option<RobberPosition>,
    pub steps_left: u8,
    pub turn: Turn,
}

impl Game {
    pub fn new(
        graph: &Graph,
        number_of_cops: u8,
        number_of_steps: u8,
        cop: Algorithm,
        robber: Algorithm,
    ) -> Game {
        let cop: Box<dyn Cop + Send> = match cop {
            Algorithm::Random => Box::new(RandomCop::new(number_of_cops)),
            Algorithm::Menace => Box::new(MenaceCop::new(number_of_cops)),
        };
        let robber: Box<dyn Robber + Send> = match robber {
            Algorithm::Random => Box::new(RandomRobber::new()),
            Algorithm::Menace => Box::new(MenaceRobber::new()),
        };
        Game {
            graph: graph.clone(),
            number_of_steps,
            cop,
            robber,
            score: [0, 0],
            cop_positions: None,
            robber_position: None,
            steps_left: number_of_steps,
            turn: Turn::Cop,
        }
    }

    pub fn update(&mut self) {
        match self.turn {
            Turn::Cop => {
                if let Some(cop_positions) = &self.cop_positions {
                    let robber_position = self.robber_position.unwrap(); // Robber position will exist as we have cop_positions and it's a cop turn.
                    let new_cop_positions =
                        self.cop.step(&self.graph, cop_positions, robber_position);
                    if new_cop_positions.contains(&robber_position) {
                        // Cop won
                        self.cop
                            .end(&self.graph, &new_cop_positions, robber_position);
                        self.robber
                            .end(&self.graph, &new_cop_positions, robber_position);
                        self.score[0] += 1;
                        self.turn = Turn::Over;
                    } else {
                        self.turn = Turn::Robber;
                    }
                    self.cop_positions = Some(new_cop_positions);
                } else {
                    self.cop_positions = Some(self.cop.start(&self.graph));
                    self.turn = Turn::Robber;
                }
            }
            Turn::Robber => {
                let cop_positions = self.cop_positions.as_ref().unwrap(); // Since it's a robber turn, cop_positions will not be None.

                let new_robber_position = if let Some(robber_position) = self.robber_position {
                    self.steps_left -= 1; // Decrease by one as robber made their move.
                    self.robber
                        .step(&self.graph, cop_positions, robber_position)
                } else {
                    // We don't decrease by one as the robber just chooses their starting position.
                    self.robber.start(&self.graph, cop_positions)
                };

                if cop_positions.contains(&new_robber_position) {
                    // Cop won
                    self.cop
                        .end(&self.graph, cop_positions, new_robber_position);
                    self.robber
                        .end(&self.graph, cop_positions, new_robber_position);
                    self.score[0] += 1;
                    self.turn = Turn::Over;
                } else if self.steps_left == 0 {
                    // Robber won
                    self.cop
                        .end(&self.graph, cop_positions, new_robber_position);
                    self.robber
                        .end(&self.graph, cop_positions, new_robber_position);
                    self.score[1] += 1;
                    self.turn = Turn::Over;
                } else {
                    self.turn = Turn::Cop;
                }
                self.robber_position = Some(new_robber_position);
            }
            Turn::Over => {
                self.cop_positions = None;
                self.robber_position = None;
                self.steps_left = self.number_of_steps;
                self.turn = Turn::Cop;
            }
        }
    }
}
