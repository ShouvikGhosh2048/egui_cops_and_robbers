use crate::game::{Algorithm, Game, GameSettings, TemplateGraph, Turn, TEMPLATE_GRAPHS};
use egui::{mutex::Mutex, Vec2};
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

struct GameAndAnimationState {
    game: Game,
    // Fields for animating moves.
    previous_cop_positions: Option<Vec<usize>>,
    previous_robber_position: Option<usize>,
    // We use egui's animate_bool_with_time for animations.
    // animation_bool is just a boolean. Everytime we want to animate a transition,
    // we set flip_animation_bool to true. Then, inside show_game, we flip animation_bool
    // (and set back flip_animation_bool to false) and pass show_game to animate_bool_with_time.
    animation_bool: bool,
    flip_animation_bool: bool,
}

impl GameAndAnimationState {
    fn new(game_settings: &GameSettings) -> Self {
        Self {
            game: Game::new(game_settings),
            previous_cop_positions: None,
            previous_robber_position: None,
            animation_bool: false,
            // We set flip_animation_bool to true initially so that show_game passes animation_bool
            // to animate_bool_with_time, thus initializing the bool value.
            flip_animation_bool: true,
        }
    }

    fn update(&mut self) {
        self.previous_cop_positions = self.game.cop_positions.clone();
        self.previous_robber_position = self.game.robber_position;
        self.flip_animation_bool = true;
        self.game.update();
    }
}

// GameHandle is a handle to a new thread created to play the game.
// It also allows us to request the new thread to perform multiple updates at a time.
struct GameHandle {
    // The game and animation state. We wrap GameAndAnimationState in an option
    // so that we can set game to hold None when we want the new thread to stop.
    game_and_animation_state: Arc<Mutex<Option<GameAndAnimationState>>>,
    // The number of games we want to compute immediately.
    number_of_immediate_games: Arc<Mutex<Option<u32>>>,
    // Handle of the new thread. We store it in an Option so that we can take it out of GameHandle
    // and call join on it to wait for the new thread to finish.
    thread_handle: Option<JoinHandle<()>>,
}

impl GameHandle {
    fn new(game_settings: &GameSettings, ctx: egui::Context) -> Self {
        let game_and_animation_state =
            Arc::new(Mutex::new(Some(GameAndAnimationState::new(game_settings))));
        let game_and_animation_state_clone = Arc::clone(&game_and_animation_state);

        let number_of_immediate_games = Arc::new(Mutex::new(None));
        let number_of_immediate_games_clone = Arc::clone(&number_of_immediate_games);

        let handle = thread::spawn(move || loop {
            let mut have_done_multiple_moves = false;

            loop {
                let games = *(number_of_immediate_games.lock());

                if let Some(games) = games {
                    {
                        let mut game_and_animation_state = game_and_animation_state.lock();

                        let mut games_till_now = 0;
                        if let Some(GameAndAnimationState { game, .. }) =
                            &mut (*game_and_animation_state)
                        {
                            while games_till_now < games {
                                // We don't need to update animation as we don't animate here, so we can just update the game.
                                game.update();
                                if game.turn == Turn::Over {
                                    games_till_now += 1;
                                }
                            }
                            // We don't need to update animation as after this update, we'll end up in a state with no players on the graph.
                            // We can just update the game.
                            game.update();
                        } else {
                            return; // There is no game, so we return.
                        }
                    }

                    let mut number_of_immediate_games = number_of_immediate_games.lock();
                    // We can unwrap as we're the only one that can decrement the count,
                    // and since we're here, the count is non zero.
                    let remaining_games = (*number_of_immediate_games).unwrap() - games;
                    *number_of_immediate_games = if remaining_games > 0 {
                        Some(remaining_games)
                    } else {
                        None
                    };

                    have_done_multiple_moves = true;
                } else if have_done_multiple_moves {
                    break;
                } else {
                    let mut game_and_animation_state = game_and_animation_state.lock();
                    if let Some(game_and_animation_state) = &mut (*game_and_animation_state) {
                        game_and_animation_state.update();
                    } else {
                        return; // There is no game, so we return.
                    }
                    break;
                }
            }

            ctx.request_repaint();
            thread::sleep(Duration::from_millis(1000));
        });
        GameHandle {
            game_and_animation_state: game_and_animation_state_clone,
            number_of_immediate_games: number_of_immediate_games_clone,
            thread_handle: Some(handle),
        }
    }
}

impl Drop for GameHandle {
    fn drop(&mut self) {
        // Set game_and_animation_state to None to inform the new thread
        // to stop playing the game.
        *(self.game_and_animation_state.lock()) = None;

        let thread_handle = self.thread_handle.take();
        if let Some(thread_handle) = thread_handle {
            thread_handle.join().unwrap();
        }
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    game_settings: GameSettings,

    #[serde(skip)]
    game_handle: Option<GameHandle>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        TemplateApp {
            game_settings: GameSettings {
                graph: TemplateGraph::Path2,
                number_of_cops: 1,
                number_of_steps: 1,
                cop: Algorithm::Random,
                robber: Algorithm::Random,
            },
            game_handle: None,
        }
    }
}

fn show_graph(ui: &mut egui::Ui, template_graph: TemplateGraph) -> egui::Response {
    let size = egui::vec2(300.0, 300.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let rect = rect.expand(visuals.expansion);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);

        let graph = &TEMPLATE_GRAPHS[template_graph as usize];

        for (i, edges) in graph.adjacency_list.iter().enumerate() {
            for &j in edges.iter() {
                ui.painter().line_segment(
                    [
                        rect.lerp(graph.vertices[i].into()),
                        rect.lerp(graph.vertices[j].into()),
                    ],
                    visuals.fg_stroke,
                );
            }
        }

        for vertex in graph.vertices.iter() {
            ui.painter().circle(
                rect.lerp(vertex.into()),
                5.0,
                visuals.fg_stroke.color,
                visuals.fg_stroke,
            );
        }
    }

    response
}

fn show_game(
    ui: &mut egui::Ui,
    template_graph: TemplateGraph,
    game_state: &mut GameAndAnimationState,
) -> egui::Response {
    ui.label(format!(
        "{} - {}",
        game_state.game.score[0], game_state.game.score[1],
    ));

    let size = egui::vec2(300.0, 300.0);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::hover());

    if game_state.flip_animation_bool {
        game_state.animation_bool = !game_state.animation_bool;
        game_state.flip_animation_bool = false;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let mut animation_distance =
            ui.ctx()
                .animate_bool_with_time(response.id, game_state.animation_bool, 0.5);
        if !game_state.animation_bool {
            animation_distance = 1.0 - animation_distance;
        }

        let visuals = ui.style().interact(&response);
        let rect = rect.expand(visuals.expansion);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);

        let graph = &TEMPLATE_GRAPHS[template_graph as usize];

        for (i, edges) in graph.adjacency_list.iter().enumerate() {
            for &j in edges.iter() {
                ui.painter().line_segment(
                    [
                        rect.lerp(graph.vertices[i].into()),
                        rect.lerp(graph.vertices[j].into()),
                    ],
                    visuals.fg_stroke,
                );
            }
        }

        for vertex in graph.vertices.iter() {
            ui.painter().circle(
                rect.lerp(vertex.into()),
                5.0,
                visuals.fg_stroke.color,
                egui::Stroke::NONE,
            );
        }

        if let Some(robber_position) = game_state.game.robber_position {
            let center;
            if let Some(previous_robber_position) = game_state.previous_robber_position {
                let previous_position: Vec2 = graph.vertices[previous_robber_position].into();
                let current_position: Vec2 = graph.vertices[robber_position].into();
                center = rect.lerp(
                    previous_position * (1.0 - animation_distance)
                        + current_position * animation_distance,
                );
            } else {
                center = rect.lerp(graph.vertices[robber_position].into());
            }
            ui.painter()
                .circle(center, 5.0, egui::Color32::BLUE, egui::Stroke::NONE);
        }

        if let Some(cop_positions) = &game_state.game.cop_positions {
            if let Some(previous_cop_positions) = &game_state.previous_cop_positions {
                for (&cop_position, &previous_cop_position) in
                    cop_positions.iter().zip(previous_cop_positions.iter())
                {
                    let previous_position: Vec2 = graph.vertices[previous_cop_position].into();
                    let current_position: Vec2 = graph.vertices[cop_position].into();
                    let center = rect.lerp(
                        previous_position * (1.0 - animation_distance)
                            + current_position * animation_distance,
                    );
                    ui.painter()
                        .circle(center, 5.0, egui::Color32::RED, egui::Stroke::NONE);
                }
            } else {
                for &cop_position in cop_positions {
                    ui.painter().circle(
                        rect.lerp(graph.vertices[cop_position].into()),
                        5.0,
                        egui::Color32::RED,
                        egui::Stroke::NONE,
                    );
                }
            }
        }
    }

    response
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        cc.egui_ctx.set_visuals(egui::Visuals::light());
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            game_settings,
            game_handle,
        } = self;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Cops and Robbers");

            if let Some(GameHandle {
                game_and_animation_state,
                number_of_immediate_games,
                ..
            }) = game_handle
            {
                if ui.button("Create new game").clicked() {
                    *game_handle = None;
                    return;
                }

                if ui.button("Play 1000 games").clicked() {
                    let mut number_of_immediate_games = number_of_immediate_games.lock();
                    *number_of_immediate_games = match *number_of_immediate_games {
                        Some(games) => Some(games + 1000),
                        None => Some(1000),
                    };
                }

                let number_of_immediate_games = number_of_immediate_games.lock();
                if number_of_immediate_games.is_some() {
                    ui.spinner();
                    show_graph(ui, game_settings.graph);
                } else {
                    let mut game_and_animation_state = game_and_animation_state.lock();
                    if let Some(game_and_animation_state) = &mut (*game_and_animation_state) {
                        show_game(ui, game_settings.graph, game_and_animation_state);
                    }
                }

                return;
            }

            let GameSettings {
                graph,
                number_of_cops,
                number_of_steps,
                cop,
                robber,
            } = game_settings;

            ui.horizontal(|ui| {
                ui.label("Graph");
                egui::ComboBox::from_id_source("Graph")
                    .selected_text(format!("{graph:?}"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(graph, TemplateGraph::Path2, "Path2");
                        ui.selectable_value(graph, TemplateGraph::Path5, "Path5");
                        ui.selectable_value(graph, TemplateGraph::Hexagon, "Hexagon");
                    });
            });
            show_graph(ui, *graph);

            ui.horizontal(|ui| {
                ui.label("Number of cops");
                egui::ComboBox::from_id_source("Number of cops")
                    .selected_text(format!("{number_of_cops}"))
                    .show_ui(ui, |ui| {
                        for i in 1..=3 {
                            ui.selectable_value(number_of_cops, i, i.to_string());
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Number of steps");
                ui.add(egui::DragValue::new(number_of_steps).clamp_range(0..=100));
            });

            ui.horizontal(|ui| {
                ui.label("Cop algorithm");
                egui::ComboBox::from_id_source("Cop algorithm")
                    .selected_text(format!("{cop:?}"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(cop, Algorithm::Random, "Random");
                        ui.selectable_value(cop, Algorithm::Menace, "Menace");
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Robber algorithm");
                egui::ComboBox::from_id_source("Robber algorithm")
                    .selected_text(format!("{robber:?}"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(robber, Algorithm::Random, "Random");
                        ui.selectable_value(robber, Algorithm::Menace, "Menace");
                    });
            });

            if ui.button("Play").clicked() {
                *game_handle = Some(GameHandle::new(game_settings, ctx.clone()));
            }
        });
    }
}