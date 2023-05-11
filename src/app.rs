use crate::game::{template_graphs, Algorithm, Cop, Game, Graph, Robber, Turn};
use egui::{
    containers::Frame,
    mutex::Mutex,
    plot::{Line, Plot, PlotBounds, PlotPoints},
    Color32, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2,
};
use std::{
    cmp::Ordering,
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

const ANIMATION_TIME: f32 = 0.5;
const COP_COLOR: Color32 = Color32::from_rgb(230, 30, 10);
const ROBBER_COLOR: Color32 = Color32::from_rgb(0, 100, 225);

#[derive(PartialEq)]
enum MenaceEditingVertex {
    None,
    Cop(usize),
    Robber,
}

struct MenaceCopViewingState {
    bag_key: Option<(Vec<usize>, usize)>,
    editing_vertex: MenaceEditingVertex,
    sort_by_counts: bool,
}

struct MenaceRobberViewingState {
    bag_key: (Vec<usize>, Option<usize>),
    editing_vertex: MenaceEditingVertex,
    sort_by_counts: bool,
}

#[derive(PartialEq)]
enum GameStatisticsView {
    Graph,
    Robber,
    Cop,
}

struct GameViewState {
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
    cop_scores: Vec<u32>,
    game_statistics_view: GameStatisticsView,
    menace_cop_viewing_state: Option<MenaceCopViewingState>,
    menace_robber_viewing_state: Option<MenaceRobberViewingState>,
}

impl GameViewState {
    fn new(
        graph: &Graph,
        number_of_cops: u8,
        number_of_steps: u8,
        cop: Algorithm,
        robber: Algorithm,
    ) -> Self {
        let menace_cop_viewing_state = if cop == Algorithm::Menace {
            Some(MenaceCopViewingState {
                bag_key: None,
                editing_vertex: MenaceEditingVertex::None,
                sort_by_counts: false,
            })
        } else {
            None
        };

        let menace_robber_viewing_state = if robber == Algorithm::Menace {
            Some(MenaceRobberViewingState {
                bag_key: (vec![0; number_of_cops as usize], None),
                editing_vertex: MenaceEditingVertex::None,
                sort_by_counts: false,
            })
        } else {
            None
        };

        Self {
            game: Game::new(graph, number_of_cops, number_of_steps, cop, robber),
            previous_cop_positions: None,
            previous_robber_position: None,
            animation_bool: false,
            // We set flip_animation_bool to true initially so that show_game passes animation_bool
            // to animate_bool_with_time, thus initializing the bool value.
            flip_animation_bool: true,
            game_statistics_view: GameStatisticsView::Cop,
            cop_scores: vec![],
            menace_cop_viewing_state,
            menace_robber_viewing_state,
        }
    }

    fn update(&mut self) {
        self.previous_cop_positions = self.game.cop_positions.clone();
        self.previous_robber_position = self.game.robber_position;
        self.flip_animation_bool = true;
        self.game.update();
        if self.game.turn == Turn::Over {
            let number_of_turns = self.game.score[0] + self.game.score[1];
            // If the number_of_turns is a square.
            if ((number_of_turns as f64).sqrt() as u32).pow(2) == number_of_turns {
                self.cop_scores.push(self.game.score[0]);
            }
        }
    }
}

// GameHandle is a handle to a new thread created to play the game.
// It also allows us to request the new thread to perform multiple updates at a time.
pub struct GameHandle {
    // The game view state. We wrap GameViewState in an option
    // so that we can set game to hold None when we want the new thread to stop.
    game_view_state: Arc<Mutex<Option<GameViewState>>>,
    // The number of games we want to compute immediately.
    number_of_immediate_games: Arc<Mutex<Option<u32>>>,
    // Handle of the new thread. We store it in an Option so that we can take it out of GameHandle
    // and call join on it to wait for the new thread to finish.
    thread_handle: Option<JoinHandle<()>>,
}

impl GameHandle {
    fn new(
        graph: &Graph,
        number_of_cops: u8,
        number_of_steps: u8,
        cop: Algorithm,
        robber: Algorithm,
        ctx: egui::Context,
    ) -> Self {
        let game_and_animation_state = Arc::new(Mutex::new(Some(GameViewState::new(
            graph,
            number_of_cops,
            number_of_steps,
            cop,
            robber,
        ))));
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
                        if let Some(game_and_animation_state) = &mut (*game_and_animation_state) {
                            while games_till_now < games {
                                game_and_animation_state.update();
                                if game_and_animation_state.game.turn == Turn::Over {
                                    games_till_now += 1;
                                }
                            }
                            game_and_animation_state.update();
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
            thread::sleep(Duration::from_secs_f32(2.0 * ANIMATION_TIME));
        });
        GameHandle {
            game_view_state: game_and_animation_state_clone,
            number_of_immediate_games: number_of_immediate_games_clone,
            thread_handle: Some(handle),
        }
    }
}

impl Drop for GameHandle {
    fn drop(&mut self) {
        // Set game_and_animation_state to None to inform the new thread
        // to stop playing the game.
        *(self.game_view_state.lock()) = None;

        let thread_handle = self.thread_handle.take();
        if let Some(thread_handle) = thread_handle {
            thread_handle.join().unwrap();
        }
    }
}

#[derive(PartialEq, Default, serde::Deserialize, serde::Serialize)]
pub enum Mode {
    #[default]
    Vertex,
    Edge,
}

#[derive(PartialEq, Default, serde::Deserialize, serde::Serialize)]
pub enum SelectedItem {
    Vertex(usize),
    Edge(usize, usize),
    #[default]
    None,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct GraphCreationState {
    mode: Mode,
    selected_item: SelectedItem,
    graph: Graph,
}

pub enum View {
    GameSettingsSelection,
    GraphCreation(GraphCreationState),
    Game(GameHandle),
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    graphs: Vec<Graph>,
    current_graph: usize,
    number_of_cops: u8,
    number_of_steps: u8,
    cop: Algorithm,
    robber: Algorithm,
    #[serde(skip)]
    view: View,
}

impl Default for TemplateApp {
    fn default() -> Self {
        TemplateApp {
            graphs: template_graphs(),
            current_graph: 0,
            number_of_cops: 1,
            number_of_steps: 1,
            cop: Algorithm::Random,
            robber: Algorithm::Random,
            view: View::GameSettingsSelection,
        }
    }
}

fn show_graph(ui: &mut egui::Ui, graph: &Graph) -> egui::Response {
    let size = egui::vec2(300.0, 300.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let rect = rect.expand(visuals.expansion);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);

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

// https://github.com/emilk/egui/blob/7215fdfb7c7407b8085d53052582dac10124bdfc/crates/egui_demo_lib/src/demo/paint_bezier.rs#L68
fn show_graph_editor(
    ui: &mut egui::Ui,
    graph_creation_state: &mut GraphCreationState,
) -> egui::Response {
    const SIZE: f32 = 300.0;
    const VERTEX_RADIUS: f32 = 5.0;

    let GraphCreationState {
        graph: Graph {
            vertices,
            adjacency_list,
            ..
        },
        selected_item,
        mode,
    } = graph_creation_state;

    let (mut response, painter) = ui.allocate_painter(Vec2::new(SIZE, SIZE), Sense::click());

    let to_screen = egui::emath::RectTransform::from_to(
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1.0, 1.0)),
        response.rect,
    );

    let mut drag_edge = None; // The shape of the edge dragged by the user in edge mode, if any.
    let mut selected_anything = false; // Has any vertex/edge been selected or is still selected?

    for i in 0..vertices.len() {
        let vertex_rect_size = Vec2::splat(2.0 * VERTEX_RADIUS);
        let vertex_in_screen = to_screen.transform_pos(vertices[i].into());
        let vertex_rect = Rect::from_center_size(vertex_in_screen, vertex_rect_size);
        let vertex_id = response.id.with(i);

        let vertex_response = ui.interact(vertex_rect, vertex_id, Sense::click_and_drag());

        if vertex_response.clicked() || vertex_response.dragged() {
            selected_anything = true;
            *selected_item = SelectedItem::Vertex(i);

            if *mode == Mode::Vertex {
                let mut vertex_pos = vertices[i].into();
                vertex_pos += vertex_response.drag_delta() / SIZE;
                vertex_pos = to_screen.from().clamp(vertex_pos);
                vertices[i] = vertex_pos.into();
            } else if let Some(mouse_pos) = vertex_response.interact_pointer_pos() {
                drag_edge = Some(Shape::line_segment(
                    [vertex_in_screen, mouse_pos],
                    Stroke::new(1.0, Color32::BLACK),
                ));
            }

            response.mark_changed();
        } else if vertex_response.drag_released() && *mode == Mode::Edge {
            if let Some(mouse_pos) = vertex_response.interact_pointer_pos() {
                for j in 0..vertices.len() {
                    if i == j {
                        continue;
                    }
                    let vertex_in_screen = to_screen.transform_pos(vertices[j].into());
                    let vertex_rect = Rect::from_center_size(vertex_in_screen, vertex_rect_size);
                    if vertex_rect.contains(mouse_pos) && !adjacency_list[i].contains(&j) {
                        adjacency_list[i].push(j);
                        adjacency_list[j].push(i);
                        selected_anything = true;
                        *selected_item = if i < j {
                            SelectedItem::Edge(i, j)
                        } else {
                            SelectedItem::Edge(j, i)
                        };

                        response.mark_changed();
                    }
                }
            }
        } else if vertex_response.clicked_elsewhere() && *selected_item == SelectedItem::Vertex(i) {
            *selected_item = SelectedItem::None;
            response.mark_changed();
        }
    }

    // Add new vertex.
    if *mode == Mode::Vertex && response.clicked_by(egui::PointerButton::Secondary) {
        if let Some(pos) = response.hover_pos() {
            *selected_item = SelectedItem::Vertex(vertices.len());
            vertices.push(to_screen.inverse().transform_pos(pos).into());
            adjacency_list.push(vec![]);

            response.mark_changed();
        }
    }

    // Select an edge.
    if !selected_anything && response.clicked() {
        if let Some(Pos2 { x, y }) = response.hover_pos() {
            for i in 0..vertices.len() {
                for &j in &adjacency_list[i] {
                    if i > j {
                        continue;
                    }

                    let Pos2 { x: x1, y: y1 } = to_screen.transform_pos(vertices[i].into());
                    let Pos2 { x: x2, y: y2 } = to_screen.transform_pos(vertices[j].into());

                    // Consider the point p on the edge from vertex i to vertex j,
                    // dividing the segment into the ratio 1 - t : t where 0 <= t <= 1.
                    // The square of the distance from p to the mouse cursor is a quadratic function.
                    // We calculate the t which minimized the square of the distance, calculate the minimum distance
                    // and then select the edge if the distance is small enough.
                    let a = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
                    let b = 2.0 * ((x1 - x2) * (x2 - x) + (y1 - y2) * (y2 - y));
                    let c = (x2 - x) * (x2 - x) + (y2 - y) * (y2 - y);
                    let t = (-b / (2.0 * a)).clamp(0.0, 1.0);
                    let distance = (a * t * t + b * t + c).sqrt();
                    if distance < 5.0 {
                        selected_anything = true;
                        *selected_item = SelectedItem::Edge(i, j);

                        response.mark_changed();
                    }
                }
            }
        }
    }

    if response.clicked() && !selected_anything {
        *selected_item = SelectedItem::None;
        response.mark_changed();
    }

    // Create the shapes

    let mut selected_vertex = None;
    let mut vertex_shapes = Vec::new();
    for (i, vertex) in vertices.iter().enumerate() {
        let vertex_in_screen = to_screen.transform_pos((*vertex).into());

        if *selected_item == SelectedItem::Vertex(i) {
            selected_vertex = Some(Shape::circle_filled(
                vertex_in_screen,
                VERTEX_RADIUS,
                Color32::BLACK,
            ));
            continue;
        }

        vertex_shapes.push(Shape::circle_filled(
            vertex_in_screen,
            VERTEX_RADIUS,
            Color32::GRAY,
        ));
    }
    if let Some(vertex) = selected_vertex {
        vertex_shapes.push(vertex);
    }

    let mut selected_edge = None;
    let mut edge_shapes = Vec::new();
    for i in 0..vertices.len() {
        for &j in &adjacency_list[i] {
            if i > j {
                continue;
            }

            let v1_in_screen = to_screen.transform_pos(vertices[i].into());
            let v2_in_screen = to_screen.transform_pos(vertices[j].into());

            if *selected_item == SelectedItem::Edge(i, j) {
                selected_edge = Some(Shape::line_segment(
                    [v1_in_screen, v2_in_screen],
                    Stroke::new(1.0, Color32::BLACK),
                ));
                continue;
            }

            edge_shapes.push(Shape::line_segment(
                [v1_in_screen, v2_in_screen],
                Stroke::new(1.0, Color32::GRAY),
            ));
        }
    }
    if let Some(edge) = selected_edge {
        edge_shapes.push(edge);
    }
    if let Some(edge) = drag_edge {
        edge_shapes.push(edge);
    }

    painter.extend(edge_shapes);
    painter.extend(vertex_shapes);

    response
}

fn show_game(ui: &mut egui::Ui, graph: &Graph, game_state: &mut GameViewState) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Score (Cop-Robber):").strong());
        ui.label(format!(
            "{} - {}",
            game_state.game.score[0], game_state.game.score[1],
        ));
    });

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
                .animate_bool_with_time(response.id, game_state.animation_bool, ANIMATION_TIME);
        if !game_state.animation_bool {
            animation_distance = 1.0 - animation_distance;
        }

        let visuals = ui.style().interact(&response);
        let rect = rect.expand(visuals.expansion);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);

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
                .circle(center, 6.0, ROBBER_COLOR, egui::Stroke::NONE);
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
                        .circle(center, 5.0, COP_COLOR, egui::Stroke::NONE);
                }
            } else {
                for &cop_position in cop_positions {
                    ui.painter().circle(
                        rect.lerp(graph.vertices[cop_position].into()),
                        5.0,
                        COP_COLOR,
                        egui::Stroke::NONE,
                    );
                }
            }
        }
    }

    response
}

fn select_graph_vertex(
    ui: &mut egui::Ui,
    vertex: &mut usize,
    graph: &Graph,
    is_cop: bool,
) -> egui::Response {
    const VERTEX_RADIUS: f32 = 5.0;

    let size = egui::vec2(300.0, 300.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let rect = rect.expand(visuals.expansion);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);

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

        for i in 0..graph.vertices.len() {
            let vertex_rect_size = Vec2::splat(2.0 * VERTEX_RADIUS);
            let vertex_in_screen = rect.lerp(graph.vertices[i].into());
            let vertex_rect = Rect::from_center_size(vertex_in_screen, vertex_rect_size);
            let vertex_id = response.id.with(i);

            let vertex_response = ui.interact(vertex_rect, vertex_id, Sense::click());

            if vertex_response.clicked() {
                *vertex = i;
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

        let vertex = graph.vertices[*vertex];
        let (radius, color) = if is_cop {
            (5.0, COP_COLOR)
        } else {
            (6.0, ROBBER_COLOR)
        };
        ui.painter()
            .circle(rect.lerp(vertex.into()), radius, color, egui::Stroke::NONE);
    }

    response
}

fn show_graph_with_cops_and_robber(
    ui: &mut egui::Ui,
    cops: Option<&[usize]>,
    robber: Option<usize>,
    graph: &Graph,
    size: f32,
) -> egui::Response {
    let cop_size = size / 60.0;
    let graph_size = egui::vec2(size, size);
    let (rect, response) = ui.allocate_exact_size(graph_size, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let rect = rect.expand(visuals.expansion);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);

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
                cop_size,
                visuals.fg_stroke.color,
                visuals.fg_stroke,
            );
        }

        if let Some(robber) = robber {
            let robber_vertex = graph.vertices[robber];
            ui.painter().circle(
                rect.lerp(robber_vertex.into()),
                1.2 * cop_size,
                ROBBER_COLOR,
                egui::Stroke::NONE,
            );
        }

        if let Some(cops) = cops {
            for &vertex in cops {
                let vertex = graph.vertices[vertex];
                ui.painter().circle(
                    rect.lerp(vertex.into()),
                    cop_size,
                    COP_COLOR,
                    egui::Stroke::NONE,
                );
            }
        }
    }

    response
}

fn game_settings_selection(
    ctx: &egui::Context,
    graphs: &Vec<Graph>,
    current_graph: &mut usize,
    number_of_cops: &mut u8,
    number_of_steps: &mut u8,
    cop: &mut Algorithm,
    robber: &mut Algorithm,
) -> Option<View> {
    let mut view = None;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Cops and Robbers");

        ui.horizontal(|ui| {
            ui.label("Graph");
            egui::ComboBox::from_id_source("Graph")
                .selected_text(graphs[*current_graph].name.clone())
                .show_ui(ui, |ui| {
                    for (i, graph) in graphs.iter().enumerate() {
                        ui.selectable_value(current_graph, i, graph.name.clone());
                    }
                });
            if ui.button("New graph").clicked() {
                view = Some(View::GraphCreation(GraphCreationState::default()));
            }
        });
        show_graph(ui, &graphs[*current_graph]);

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
            view = Some(View::Game(GameHandle::new(
                &graphs[*current_graph],
                *number_of_cops,
                *number_of_steps,
                *cop,
                *robber,
                ctx.clone(),
            )));
        }
    });

    view
}

fn graph_creation(
    ctx: &egui::Context,
    graph_creation_state: &mut GraphCreationState,
    graphs: &mut Vec<Graph>,
    current_graph: &mut usize,
) -> Option<View> {
    let mut view = None;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Cops and Robbers");

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("Graph name");
            ui.add(
                egui::TextEdit::singleline(&mut graph_creation_state.graph.name)
                    .desired_width(100.0),
            );
        });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.selectable_value(&mut graph_creation_state.mode, Mode::Vertex, "Vertex mode");
            ui.selectable_value(&mut graph_creation_state.mode, Mode::Edge, "Edge mode");
            if ui.button("Delete").clicked() {
                match graph_creation_state.selected_item {
                    SelectedItem::Vertex(i) => {
                        graph_creation_state.graph.vertices.remove(i);
                        graph_creation_state.graph.adjacency_list.remove(i);

                        // We will go through the adjaceny list.
                        // We will remove all occurences of i and relabel any vertex v greater than i as v - 1.
                        graph_creation_state
                            .graph
                            .adjacency_list
                            .iter_mut()
                            .for_each(|list| {
                                let mut removed_vertex_position = None;
                                for (index, v) in list.iter_mut().enumerate() {
                                    match (*v).cmp(&i) {
                                        Ordering::Greater => *v -= 1,
                                        Ordering::Equal => removed_vertex_position = Some(index),
                                        Ordering::Less => {}
                                    }
                                }
                                if let Some(index) = removed_vertex_position {
                                    list.remove(index);
                                }
                            });
                        graph_creation_state.selected_item = SelectedItem::None;
                    }
                    SelectedItem::Edge(i, j) => {
                        let adjaceny_list_i = &mut graph_creation_state.graph.adjacency_list[i];
                        for k in 0..adjaceny_list_i.len() {
                            if adjaceny_list_i[k] == j {
                                adjaceny_list_i.remove(k);
                                break;
                            }
                        }

                        let adjaceny_list_j = &mut graph_creation_state.graph.adjacency_list[j];
                        for k in 0..adjaceny_list_j.len() {
                            if adjaceny_list_j[k] == i {
                                adjaceny_list_j.remove(k);
                                break;
                            }
                        }

                        graph_creation_state.selected_item = SelectedItem::None;
                    }
                    _ => {}
                }
            }
        });

        Frame::canvas(ui.style()).show(ui, |ui| {
            show_graph_editor(ui, graph_creation_state);
        });

        ui.horizontal(|ui| {
            if ui.button("Create").clicked() {
                if graph_creation_state.graph.name.is_empty()
                    || graph_creation_state.graph.vertices.is_empty()
                {
                    return;
                }
                graphs.push(graph_creation_state.graph.clone());
                *current_graph = graphs.len() - 1;
                view = Some(View::GameSettingsSelection);
            }
            if ui.button("Cancel").clicked() {
                view = Some(View::GameSettingsSelection);
            }
        });
    });

    view
}

fn game_details(
    ctx: &egui::Context,
    game_and_animation_state: &mut Arc<Mutex<Option<GameViewState>>>,
    number_of_immediate_games: &mut Arc<Mutex<Option<u32>>>,
    number_of_cops: u8,
) {
    egui::SidePanel::right("Details")
        .exact_width(350.0)
        .resizable(false)
        .show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink([false, true]).show(ui, |ui| {
                let number_of_immediate_games = number_of_immediate_games.lock();

                // Check if their are games to be computed
                // - if so, we shouldn't get the game_and_animation_state lock right now,
                // as the computation thread may be locking it.
                if number_of_immediate_games.is_some() {
                    ui.spinner();
                    return;
                }

                let mut game_and_animation_state = game_and_animation_state.lock();
                if let Some(game_and_animation_state) = &mut (*game_and_animation_state)
                {
                    let GameViewState {
                        game,
                        game_statistics_view,
                        menace_cop_viewing_state,
                        menace_robber_viewing_state,
                        cop_scores,
                        ..
                    } = game_and_animation_state;

                    ui.horizontal(|ui| {
                        ui.selectable_value(game_statistics_view, GameStatisticsView::Cop, "Cop");
                        ui.selectable_value(game_statistics_view, GameStatisticsView::Robber, "Robber");
                        ui.selectable_value(game_statistics_view, GameStatisticsView::Graph, "Graph");
                    });

                    match game_statistics_view {
                        GameStatisticsView::Cop => {
                            match &mut game.cop {
                                Cop::Random(_) => {
                                    ui.label(RichText::new("Random cop").strong());
                                }
                                Cop::Menace(cop) => {
                                    let MenaceCopViewingState {
                                        bag_key,
                                        editing_vertex,
                                        sort_by_counts,
                                    } = menace_cop_viewing_state.as_mut().unwrap();

                                    ui.label(RichText::new("Menace cop").strong());

                                    ui.add_space(10.0);

                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Bag:").strong());

                                        // start_bag is a temporary bool we can change,
                                        // we use it to change whether we're currently viewing
                                        // the start bag or a non start bag.
                                        let mut start_bag = bag_key.is_none();
                                        let selected_text =
                                            if start_bag { "Start" } else { "Non start" };
                                        egui::ComboBox::from_id_source("Cop bag type")
                                            .selected_text(selected_text)
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut start_bag, true, "Start");
                                                ui.selectable_value(
                                                    &mut start_bag,
                                                    false,
                                                    "Non start",
                                                );
                                            });
                                        // if start_bag changes, we update menace_cop_viewing_bag
                                        if start_bag {
                                            if bag_key.is_some() {
                                                *bag_key = None;
                                            }
                                        } else if bag_key.is_none() {
                                            *bag_key = Some((vec![0; number_of_cops as usize], 0));
                                        }
                                    });

                                    // If we are viewing a non-start bag, we allow the user to select the bag.
                                    if let Some((cops, robber)) = bag_key {
                                        ui.horizontal(|ui| {
                                            ui.selectable_value(
                                                editing_vertex,
                                                MenaceEditingVertex::None,
                                                "View",
                                            );
                                            for i in 0..cops.len() {
                                                ui.selectable_value(
                                                    editing_vertex,
                                                    MenaceEditingVertex::Cop(i),
                                                    format!("Edit cop {i}"),
                                                );
                                            }
                                            ui.selectable_value(
                                                editing_vertex,
                                                MenaceEditingVertex::Robber,
                                                "Edit robber",
                                            );
                                        });

                                        match editing_vertex {
                                            MenaceEditingVertex::None => {
                                                show_graph_with_cops_and_robber(
                                                    ui,
                                                    Some(cops),
                                                    Some(*robber),
                                                    &game.graph,
                                                    300.0,
                                                );
                                            }
                                            MenaceEditingVertex::Cop(i) => {
                                                select_graph_vertex(
                                                    ui,
                                                    &mut cops[*i],
                                                    &game.graph,
                                                    true,
                                                );
                                            }
                                            MenaceEditingVertex::Robber => {
                                                select_graph_vertex(
                                                    ui,
                                                    robber,
                                                    &game.graph,
                                                    false,
                                                );
                                            }
                                        }
                                    }

                                    ui.add_space(10.0);

                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Moves:").strong());
                                        ui.toggle_value(sort_by_counts, "Sort moves");
                                    });

                                    let bag = cop.bags.get(bag_key);
                                    if let Some((cops, robber)) = bag_key {
                                        // Non start position
                                        match bag {
                                            None => {
                                                let mut choices = 1;
                                                for cop in cops.iter() {
                                                    choices *= game.graph.adjacency_list[*cop]
                                                        .len()
                                                        + 1;
                                                }
                                                for mut choice in 0..choices {
                                                    let mut position = vec![];
                                                    for &cop in cops.iter() {
                                                        let neighbours =
                                                            &game.graph.adjacency_list[cop];
                                                        let new_cop_position =
                                                            choice % (neighbours.len() + 1);
                                                        if new_cop_position == neighbours.len()
                                                        {
                                                            position.push(cop);
                                                        } else {
                                                            position.push(
                                                                neighbours[new_cop_position],
                                                            );
                                                        }
                                                        choice /= neighbours.len() + 1;
                                                    }
                                                    ui.horizontal(|ui| {
                                                        show_graph_with_cops_and_robber(
                                                            ui,
                                                            Some(&position),
                                                            Some(*robber),
                                                            &game.graph,
                                                            180.0,
                                                        );
                                                        ui.label("50");
                                                    });
                                                }
                                            }
                                            Some(bag) => {
                                                let positions_and_counts = bag
                                                    .counts
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(mut choice, count)| {
                                                        let mut position = vec![];
                                                        for &cop in cops.iter() {
                                                            let neighbours =
                                                                &game.graph.adjacency_list[cop];
                                                            let new_cop_position =
                                                                choice % (neighbours.len() + 1);
                                                            if new_cop_position
                                                                == neighbours.len()
                                                            {
                                                                position.push(cop);
                                                            } else {
                                                                position.push(
                                                                    neighbours
                                                                        [new_cop_position],
                                                                );
                                                            }
                                                            choice /= neighbours.len() + 1;
                                                        }
                                                        (position, count)
                                                    });
                                                if *sort_by_counts {
                                                    let mut positions_and_counts =
                                                        positions_and_counts
                                                            .collect::<Vec<_>>();
                                                    positions_and_counts
                                                        .sort_by_key(|(_, count)| **count);
                                                    for (position, count) in
                                                        positions_and_counts.iter().rev()
                                                    {
                                                        ui.horizontal(|ui| {
                                                            show_graph_with_cops_and_robber(
                                                                ui,
                                                                Some(position),
                                                                Some(*robber),
                                                                &game.graph,
                                                                180.0,
                                                            );
                                                            ui.label(count.to_string());
                                                        });
                                                    }
                                                } else {
                                                    for (position, count) in
                                                        positions_and_counts
                                                    {
                                                        ui.horizontal(|ui| {
                                                            show_graph_with_cops_and_robber(
                                                                ui,
                                                                Some(&position),
                                                                Some(*robber),
                                                                &game.graph,
                                                                180.0,
                                                            );
                                                            ui.label(count.to_string());
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        match bag {
                                            None => {
                                                let choices = game
                                                    .graph
                                                    .vertices
                                                    .len()
                                                    .pow(number_of_cops as u32);
                                                for mut choice in 0..choices {
                                                    let mut position = vec![];
                                                    for _ in 0..number_of_cops {
                                                        position.push(
                                                            choice % game.graph.vertices.len(),
                                                        );
                                                        choice /= game.graph.vertices.len();
                                                    }
                                                    ui.horizontal(|ui| {
                                                        show_graph_with_cops_and_robber(
                                                            ui,
                                                            Some(&position),
                                                            None,
                                                            &game.graph,
                                                            180.0,
                                                        );
                                                        ui.label("50");
                                                    });
                                                }
                                            }
                                            Some(bag) => {
                                                let positions_and_counts = bag
                                                    .counts
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(mut choice, count)| {
                                                        let mut position = vec![];
                                                        for _ in 0..number_of_cops {
                                                            position.push(
                                                                choice
                                                                    % game.graph.vertices.len(),
                                                            );
                                                            choice /= game.graph.vertices.len();
                                                        }
                                                        (position, count)
                                                    });
                                                if *sort_by_counts {
                                                    let mut positions_and_counts =
                                                        positions_and_counts
                                                            .collect::<Vec<_>>();
                                                    positions_and_counts
                                                        .sort_by_key(|(_, count)| **count);
                                                    for (position, count) in
                                                        positions_and_counts.iter().rev()
                                                    {
                                                        ui.horizontal(|ui| {
                                                            show_graph_with_cops_and_robber(
                                                                ui,
                                                                Some(position),
                                                                None,
                                                                &game.graph,
                                                                180.0,
                                                            );
                                                            ui.label(count.to_string());
                                                        });
                                                    }
                                                } else {
                                                    for (position, count) in
                                                        positions_and_counts
                                                    {
                                                        ui.horizontal(|ui| {
                                                            show_graph_with_cops_and_robber(
                                                                ui,
                                                                Some(&position),
                                                                None,
                                                                &game.graph,
                                                                180.0,
                                                            );
                                                            ui.label(count.to_string());
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        GameStatisticsView::Robber => {
                            match &mut game.robber {
                                Robber::Random(_) => {
                                    ui.label(RichText::new("Random robber").strong());
                                }
                                Robber::Menace(robber) => {
                                    ui.label(RichText::new("Menace robber").strong());

                                    ui.add_space(10.0);

                                    let MenaceRobberViewingState {
                                        bag_key,
                                        editing_vertex,
                                        sort_by_counts,
                                    } = menace_robber_viewing_state.as_mut().unwrap();

                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Bag:").strong());

                                        // start_bag is a temporary bool we can change,
                                        // we use it to change whether we're currently viewing
                                        // a start bag or a non start bag.
                                        let mut start_bag = bag_key.1.is_none();
                                        let selected_text =
                                            if start_bag { "Start" } else { "Non start" };
                                        egui::ComboBox::from_id_source("Robber bag type")
                                            .selected_text(selected_text)
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut start_bag, true, "Start");
                                                ui.selectable_value(
                                                    &mut start_bag,
                                                    false,
                                                    "Non start",
                                                );
                                            });
                                        // if start_bag changes, we update menace_cop_viewing_bag
                                        if start_bag {
                                            if bag_key.1.is_some() {
                                                *bag_key = (vec![0; number_of_cops as usize], None);
                                                *editing_vertex = MenaceEditingVertex::None;
                                            }
                                        } else if bag_key.1.is_none() {
                                            *bag_key = (vec![0; number_of_cops as usize], Some(0));
                                            *editing_vertex = MenaceEditingVertex::None;
                                        }
                                    });

                                    // We allow the user to select the bag.
                                    let cops = &mut bag_key.0;
                                    ui.horizontal(|ui| {
                                        ui.selectable_value(
                                            editing_vertex,
                                            MenaceEditingVertex::None,
                                            "View",
                                        );
                                        for i in 0..cops.len() {
                                            ui.selectable_value(
                                                editing_vertex,
                                                MenaceEditingVertex::Cop(i),
                                                format!("Edit cop {i}"),
                                            );
                                        }
                                        if bag_key.1.is_some() {
                                            ui.selectable_value(
                                                editing_vertex,
                                                MenaceEditingVertex::Robber,
                                                "Edit robber",
                                            );
                                        }
                                    });
                                    match editing_vertex {
                                        MenaceEditingVertex::None => {
                                            show_graph_with_cops_and_robber(
                                                ui,
                                                Some(cops),
                                                bag_key.1,
                                                &game.graph,
                                                300.0,
                                            );
                                        }
                                        MenaceEditingVertex::Cop(i) => {
                                            select_graph_vertex(
                                                ui,
                                                &mut cops[*i],
                                                &game.graph,
                                                true,
                                            );
                                        }
                                        MenaceEditingVertex::Robber => {
                                            if let Some(robber) = &mut bag_key.1 {
                                                select_graph_vertex(
                                                    ui,
                                                    robber,
                                                    &game.graph,
                                                    false,
                                                );
                                            } else {
                                                // Shouldn't be editing robber, so we change editing vertex.
                                                *editing_vertex = MenaceEditingVertex::None;
                                            }
                                        }
                                    }

                                    ui.add_space(10.0);

                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Moves:").strong());
                                        ui.toggle_value(sort_by_counts, "Sort moves");
                                    });

                                    let bag = robber.bags.get(bag_key);
                                    let cops = &bag_key.0;
                                    if let Some(robber) = bag_key.1 {
                                        // Non start position
                                        match bag {
                                            None => {
                                                let neighbours = &game.graph.adjacency_list[robber];
                                                for &neighbour in neighbours {
                                                    ui.horizontal(|ui| {
                                                        show_graph_with_cops_and_robber(
                                                            ui,
                                                            Some(cops),
                                                            Some(neighbour),
                                                            &game.graph,
                                                            180.0,
                                                        );
                                                        ui.label("50");
                                                    });
                                                }
                                                ui.horizontal(|ui| {
                                                    show_graph_with_cops_and_robber(
                                                        ui,
                                                        Some(cops),
                                                        Some(robber),
                                                        &game.graph,
                                                        180.0,
                                                    );
                                                    ui.label("50");
                                                });
                                            }
                                            Some(bag) => {
                                                let positions_and_counts = bag
                                                    .counts
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(choice, count)| {
                                                        let neighbours = &game.graph.adjacency_list[robber];
                                                        let position = if choice == neighbours.len() {
                                                            robber
                                                        } else {
                                                            neighbours[choice]
                                                        };
                                                        (position, count)
                                                    });
                                                if *sort_by_counts {
                                                    let mut positions_and_counts =
                                                        positions_and_counts
                                                            .collect::<Vec<_>>();
                                                    positions_and_counts
                                                        .sort_by_key(|(_, count)| **count);
                                                    for &(position, count) in
                                                        positions_and_counts.iter().rev()
                                                    {
                                                        ui.horizontal(|ui| {
                                                            show_graph_with_cops_and_robber(
                                                                ui,
                                                                Some(cops),
                                                                Some(position),
                                                                &game.graph,
                                                                180.0,
                                                            );
                                                            ui.label(count.to_string());
                                                        });
                                                    }
                                                } else {
                                                    for (position, count) in
                                                        positions_and_counts
                                                    {
                                                        ui.horizontal(|ui| {
                                                            show_graph_with_cops_and_robber(
                                                                ui,
                                                                Some(cops),
                                                                Some(position),
                                                                &game.graph,
                                                                180.0,
                                                            );
                                                            ui.label(count.to_string());
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        match bag {
                                            None => {
                                                for position in 0..game.graph.vertices.len() {
                                                    ui.horizontal(|ui| {
                                                        show_graph_with_cops_and_robber(
                                                            ui,
                                                            Some(cops),
                                                            Some(position),
                                                            &game.graph,
                                                            180.0,
                                                        );
                                                        ui.label("50");
                                                    });
                                                }
                                            }
                                            Some(bag) => {
                                                let positions_and_counts = bag
                                                    .counts
                                                    .iter()
                                                    .enumerate();
                                                if *sort_by_counts {
                                                    let mut positions_and_counts =
                                                        positions_and_counts
                                                            .collect::<Vec<_>>();
                                                    positions_and_counts
                                                        .sort_by_key(|(_, count)| **count);
                                                    for (position, count) in
                                                        positions_and_counts.iter().rev()
                                                    {
                                                        ui.horizontal(|ui| {
                                                            show_graph_with_cops_and_robber(
                                                                ui,
                                                                Some(cops),
                                                                Some(*position),
                                                                &game.graph,
                                                                180.0,
                                                            );
                                                            ui.label(count.to_string());
                                                        });
                                                    }
                                                } else {
                                                    for (position, count) in
                                                        positions_and_counts
                                                    {
                                                        ui.horizontal(|ui| {
                                                            show_graph_with_cops_and_robber(
                                                                ui,
                                                                Some(cops),
                                                                Some(position),
                                                                &game.graph,
                                                                180.0,
                                                            );
                                                            ui.label(count.to_string());
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        GameStatisticsView::Graph => {
                            let half_line_points: PlotPoints = [[0.0, 0.5], [1.0, 0.5]].into_iter().collect();
                            let half_line = Line::new(half_line_points).color(Color32::BLACK);

                            let number_of_scores = cop_scores.len().pow(2) as f64;
                            let score_points = cop_scores.iter().enumerate().map(|(i, &score)| {
                                let number_of_matches = (i + 1).pow(2) as f64;
                                let cop_win_fraction = (score as f64) / number_of_matches;
                                [number_of_matches / number_of_scores, cop_win_fraction]
                            });
                            let score_points: PlotPoints = [[0.0_f64, 0.5]].into_iter().chain(score_points).collect();
                            let score_line = Line::new(score_points);

                            Plot::new("Cop wins")
                                        .view_aspect(1.0)
                                        .allow_drag(false)
                                        .allow_scroll(false)
                                        .allow_zoom(false)
                                        .allow_boxed_zoom(false)
                                        .width(330.0)
                                        .show(ui, |plot_ui|{
                                            plot_ui.set_plot_bounds(PlotBounds::from_min_max([0.0, 0.0], [1.0, 1.0]));
                                            plot_ui.line(score_line);
                                            plot_ui.line(half_line)
                                        });
                            ui.label("The fraction of cop wins (evaluated after every perfect square number of matches).");
                        }
                    }
                }
            });
        });
}

fn game(
    ctx: &egui::Context,
    game_handle: &mut GameHandle,
    graphs: &Vec<Graph>,
    current_graph: usize,
    number_of_cops: u8,
) -> Option<View> {
    let mut view = None;

    let GameHandle {
        game_view_state: game_and_animation_state,
        number_of_immediate_games,
        ..
    } = game_handle;

    game_details(
        ctx,
        game_and_animation_state,
        number_of_immediate_games,
        number_of_cops,
    );

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Cops and Robbers");

        ui.horizontal(|ui| {
            if ui.button("Create new game").clicked() {
                view = Some(View::GameSettingsSelection);
                return;
            }

            if ui.button("Play 1000 games").clicked() {
                let mut number_of_immediate_games = number_of_immediate_games.lock();
                *number_of_immediate_games = match *number_of_immediate_games {
                    Some(games) => Some(games + 1000),
                    None => Some(1000),
                };
            }
        });

        let number_of_immediate_games = number_of_immediate_games.lock();
        if number_of_immediate_games.is_some() {
            ui.spinner();
            show_graph(ui, &graphs[current_graph]);
        } else {
            let mut game_and_animation_state = game_and_animation_state.lock();
            if let Some(game_and_animation_state) = &mut (*game_and_animation_state) {
                show_game(ui, &graphs[current_graph], game_and_animation_state);
            }
        }
    });

    view
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
            graphs,
            current_graph,
            number_of_cops,
            number_of_steps,
            cop,
            robber,
            view,
        } = self;

        let new_view = match view {
            View::GameSettingsSelection => game_settings_selection(
                ctx,
                graphs,
                current_graph,
                number_of_cops,
                number_of_steps,
                cop,
                robber,
            ),
            View::GraphCreation(graph_creation_state) => {
                graph_creation(ctx, graph_creation_state, graphs, current_graph)
            }
            View::Game(game_handle) => {
                game(ctx, game_handle, graphs, *current_graph, *number_of_cops)
            }
        };

        if let Some(new_view) = new_view {
            *view = new_view;
        }
    }
}
