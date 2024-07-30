use std::time::Duration;

use bevy::{
    app::{PostUpdate, Startup, Update},
    color::Color,
    core_pipeline::core_2d::Camera2dBundle,
    ecs::{
        change_detection::Res,
        component::Component,
        entity::Entity,
        event::{Event, EventWriter},
        query::{With, Without},
        schedule::IntoSystemConfigs,
        system::{Commands, Resource},
    },
    input::{keyboard::KeyCode, ButtonInput},
    math::Vec3,
    prelude::{default, App, EventReader, PluginGroup, Query, ResMut},
    render::camera::ClearColor,
    sprite::SpriteBundle,
    time::common_conditions::on_timer,
    transform::components::Transform,
    window::{PrimaryWindow, Window, WindowPlugin, WindowResolution},
    DefaultPlugins,
};
use rand::random;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.04)))
        .insert_resource(SnakeSegments::default())
        .add_event::<GrowthEvent>()
        .add_event::<GameOverEvent>()
        .add_systems(Startup, (setup_camera, spawn_snake))
        .add_systems(
            Update,
            (
                snake_direction.before(snake_movement),
                snake_movement.run_if(on_timer(Duration::from_millis(150))),
                snake_eating.after(snake_movement),
                snake_growing.after(snake_eating),
                game_over.after(snake_eating).after(snake_movement),
                spawn_food.run_if(on_timer(Duration::from_millis(1500))),
            ),
        )
        .add_systems(PostUpdate, (size_scaling, position_translation))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Snake!".to_string(),
                resolution: WindowResolution::new(800., 800.),
                ..default()
            }),
            ..default()
        }))
        .run();
}

// SETUP

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

// SNAKE

const SNAKE_HEAD_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);

#[derive(Component)]
struct SnakeHead {
    direction: Direction,
}

fn spawn_snake(mut commands: Commands, mut segments: ResMut<SnakeSegments>) {
    *segments = SnakeSegments(vec![
        commands
            .spawn(SpriteBundle {
                sprite: bevy::sprite::Sprite {
                    color: SNAKE_HEAD_COLOR,
                    ..default()
                },
                ..default()
            })
            .insert(SnakeHead {
                direction: Direction::Up,
            })
            .insert(Size::square(0.8))
            .insert(Position { x: 3, y: 3 })
            .id(),
        spawn_snake_segment(commands, Position { x: 3, y: 2 }),
    ])
}
// SNAKE SEGMENTS

const SNAKE_SEGMENT_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);

#[derive(Component)]
struct SnakeSegment;

#[derive(Resource, Default)]
struct SnakeSegments(Vec<Entity>);

fn spawn_snake_segment(mut commands: Commands, position: Position) -> Entity {
    commands
        .spawn(SpriteBundle {
            sprite: bevy::sprite::Sprite {
                color: SNAKE_SEGMENT_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(SnakeSegment)
        .insert(Size::square(0.65))
        .insert(position)
        .id()
}

// SNAKE MOVEMENT

#[derive(PartialEq, Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}
impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Right => Self::Left,
            Self::Left => Self::Right,
        }
    }
}

fn snake_direction(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut head_positions: Query<&mut SnakeHead>,
) {
    for mut snake_head in head_positions.iter_mut() {
        let next_dir = if keyboard_input.pressed(KeyCode::ArrowLeft) {
            Direction::Left
        } else if keyboard_input.pressed(KeyCode::ArrowRight) {
            Direction::Right
        } else if keyboard_input.pressed(KeyCode::ArrowUp) {
            Direction::Up
        } else if keyboard_input.pressed(KeyCode::ArrowDown) {
            Direction::Down
        } else {
            snake_head.direction
        };
        if next_dir != snake_head.direction.opposite() {
            snake_head.direction = next_dir;
        }
    }
}

fn snake_movement(
    segments: Res<SnakeSegments>,
    mut game_over_writer: EventWriter<GameOverEvent>,
    mut heads: Query<(&SnakeHead, Entity)>,
    mut positions: Query<&mut Position>,
) {
    if let Some((snake_head, snake_head_entity)) = heads.iter_mut().next() {
        // Get original positions of all segments, including head
        let segment_positions: Vec<Position> = segments
            .0
            .iter()
            .map(|e| *positions.get(*e).unwrap())
            .collect();

        // Update the head to it's new position
        let mut head_position = positions.get_mut(snake_head_entity).unwrap();
        match snake_head.direction {
            Direction::Right => head_position.x += 1,
            Direction::Left => head_position.x -= 1,
            Direction::Up => head_position.y += 1,
            Direction::Down => head_position.y -= 1,
        }

        // If we're out of bound, game over
        if head_position.x < 0
            || head_position.y < 0
            || head_position.y > ARENA_HEIGHT as i32
            || head_position.x > ARENA_WIDTH as i32
            || segment_positions.contains(&head_position)
        {
            game_over_writer.send(GameOverEvent);
        }

        // Get the child segments of the snake
        let child_segments = segments.0.iter().skip(1);

        // Pair the original positions with the children, leaving off the
        // furthest away position. Then set each segement to the previous
        // position of the  next closets segment to the head
        segment_positions.iter().zip(child_segments).for_each(
            |(parent_seg_pos, child_seg_entity)| {
                *positions.get_mut(*child_seg_entity).unwrap() = *parent_seg_pos;
            },
        );
    }
}

// SNAKE EATING

#[derive(Event)]
struct GrowthEvent;

fn snake_eating(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    foods: Query<(&Position, Entity), With<Food>>,
    heads: Query<&Position, With<SnakeHead>>,
) {
    if let Some(snake_head_pos) = heads.iter().next() {
        for (food_pos, food_ent) in foods.iter() {
            if food_pos == snake_head_pos {
                commands.entity(food_ent).despawn();
                growth_writer.send(GrowthEvent);
            }
        }
    }
}

fn snake_growing(
    commands: Commands,
    mut growth_reader: EventReader<GrowthEvent>,
    mut segments: ResMut<SnakeSegments>,
    segments_positions: Query<(&Position, Entity), With<SnakeSegment>>,
) {
    if growth_reader.read().next().is_some() {
        let last_segment_ent = segments.0.last().unwrap();
        let (last_segment_pos, _) = segments_positions
            .iter()
            .find(|(_, ent)| ent == last_segment_ent)
            .unwrap();
        segments
            .0
            .push(spawn_snake_segment(commands, *last_segment_pos))
    }
}

// FOOD

const FOOD_COLOR: Color = Color::srgb(1.0, 0.0, 1.0);

#[derive(Component)]
struct Food;

fn spawn_food(mut commands: Commands, snake_positions: Query<&Position, Without<Food>>) {
    fn gen_pos() -> Position {
        Position {
            x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
            y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
        }
    }

    let mut next_pos = gen_pos();
    let positions_vec: Vec<&Position> = snake_positions.iter().collect();
    while positions_vec.contains(&&next_pos) {
        next_pos = gen_pos();
    }

    commands
        .spawn(SpriteBundle {
            sprite: bevy::sprite::Sprite {
                color: FOOD_COLOR,
                ..default()
            },
            transform: bevy::transform::components::Transform {
                scale: Vec3::new(10.0, 10.0, 10.0),
                ..default()
            },
            ..default()
        })
        .insert(Food)
        .insert(Size::square(0.8))
        .insert(Position {
            x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
            y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
        });
}

// GAME OVER

#[derive(Event)]
struct GameOverEvent;

fn game_over(
    mut commands: Commands,
    mut game_over_reader: EventReader<GameOverEvent>,
    entities: Query<Entity, With<Position>>,
    mut segments_res: ResMut<SnakeSegments>,
) {
    if game_over_reader.read().next().is_some() {
        for ent in entities.iter() {
            commands.entity(ent).despawn();
        }
        segments_res.0 = vec![];
        spawn_snake(commands, segments_res);
    }
}

// ARENA / SCALING

const ARENA_WIDTH: u32 = 20;
const ARENA_HEIGHT: u32 = 20;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}
impl Size {
    pub fn square(x: f32) -> Size {
        Size {
            width: x,
            height: x,
        }
    }
}

fn size_scaling(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut entites_to_scale: Query<(&Size, &mut Transform)>,
) {
    let window = windows.single();
    for (size, mut transform) in entites_to_scale.iter_mut() {
        transform.scale = Vec3::new(
            size.width / ARENA_WIDTH as f32 * window.width() as f32,
            size.height / ARENA_WIDTH as f32 * window.height() as f32,
            1.0,
        );
    }
}

fn position_translation(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut entites_to_scale: Query<(&Position, &mut Transform)>,
) {
    let window = windows.single();

    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }

    for (pos, mut transform) in entites_to_scale.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width(), ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height(), ARENA_HEIGHT as f32),
            1.0,
        );
    }
}
