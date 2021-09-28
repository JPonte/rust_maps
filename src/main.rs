use bevy::{
    prelude::*,
    render::camera::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use futures_lite::future;

mod terrain_mesh;
use terrain_mesh::*;

mod coord_utils;
use coord_utils::*;

mod camera_utils;
use camera_utils::*;

mod map_services;
use map_services::*;

#[derive(Debug, Clone, Copy)]
struct UserPosition {
    lat: f64,
    lon: f64,
    zoom: u32,
}

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            title: "Rust Maps".to_string(),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
        .insert_resource(UiState {
            detail_level: 10,
            lat: "38.272688".to_string(),
            lon: "-120.234375".to_string(),
        })
        .add_event::<MouseEvents>()
        .add_plugin(EguiPlugin)
        .add_system(controls.system())
        .add_startup_system(setup.system())
        .add_system(emit_mouse_events.system())
        .add_system(camera_motion_system.system())
        .add_system(handle_tasks.system())
        .add_system(on_camera_updated.system())
        .add_system(on_user_position_updated.system())
        .run();
}

fn handle_tasks(
    mut commands: Commands,
    mut query_tasks: Query<(Entity, &mut Task<TileInfo>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    current_tiles: Query<(Entity, &TileInfo)>,
    user_position_query: Query<&UserPosition>,
) {
    let user_pos = user_position_query.single().unwrap();
    for (tile_entity, tile) in current_tiles.iter() {
        if tile.z != user_pos.zoom || tile.base_lat != user_pos.lat || tile.base_lon != user_pos.lon {
            commands.entity(tile_entity).despawn();
        }
    }

    for (entity, mut task) in query_tasks.iter_mut() {
        if let Some(tile_info) = future::block_on(future::poll_once(&mut *task)) {
            if tile_info.z == user_pos.zoom {
                setup_terrain(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &asset_server,
                    tile_info,
                );
            }

            commands.entity(entity).remove::<Task<TileInfo>>().despawn();
        }
    }
}

fn setup(mut commands: Commands) {
    let mut camera_transform = Transform::from_translation(Vec3::new(-50., 50., -50.));
    camera_transform.look_at(Vec3::new(0., 0., 0.), Vec3::Y);

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: camera_transform,
            perspective_projection: PerspectiveProjection {
                far: 2000.,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(OrbitCamera {
            x: 45.,
            y: 30.,
            zoom: 100.,
        });

    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(0., 50., 0.)),
        light: Light {
            intensity: 10000.,
            range: 5000.,
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn().insert(UserPosition {
        lat: 38.272688,
        lon: -120.234375,
        zoom: 10,
    });
}

fn on_camera_updated(ui_state: Res<UiState>, mut user_position_query: Query<&mut UserPosition>) {
    for mut user_position in user_position_query.iter_mut() {
        if user_position.zoom != ui_state.detail_level {
            user_position.zoom = ui_state.detail_level;
        }
        if let Ok(ui_lat) = ui_state.lat.parse::<f64>() {
            if user_position.lat != ui_lat {
                user_position.lat = ui_lat;
            }
        }
        if let Ok(ui_lon) = ui_state.lon.parse::<f64>() {
            if user_position.lon != ui_lon {
                user_position.lon = ui_lon;
            }
        }
    }
}

fn on_user_position_updated(
    query: Query<&UserPosition, Changed<UserPosition>>,
    mut commands: Commands,
    thread_pool: Res<AsyncComputeTaskPool>,
) {
    if let Ok(user_pos) = query.single() {
        println!("{:?}", user_pos);

        let min_zoom = 9;
        let (top_x, top_y) = deg2num(user_pos.lat, user_pos.lon, user_pos.zoom);
        let lat = user_pos.lat;
        let lon = user_pos.lon;
        let z = user_pos.zoom;

        if user_pos.zoom > min_zoom {
            let n = user_pos.zoom - min_zoom;
            for x_i in 0..(n * n) {
                for y_i in 0..(n * n) {
                    let x = top_x + x_i;
                    let y = top_y + y_i;

                    let task = thread_pool.spawn(async move {
                        let topo_filename = async_compat::Compat::new(async {
                            get_arcgis_topo_tile(x, y, z).await
                        })
                        .await
                        .unwrap_or("".to_string());

                        let image_filename = async_compat::Compat::new(async {
                            get_arcgis_image_tile(x, y, z).await
                        })
                        .await
                        .unwrap_or("".to_string());

                        TileInfo {
                            base_lat: lat,
                            base_lon: lon,
                            x_offset: x_i,
                            y_offset: y_i,
                            z,
                            topo_filename,
                            image_filename,
                        }
                    });
                    commands.spawn().insert(task);
                }
            }
        }
    }
}

struct UiState {
    detail_level: u32,
    lat: String,
    lon: String,
}

fn controls(egui_context: ResMut<EguiContext>, mut ui_state: ResMut<UiState>) {
    egui::Window::new("Settings").show(egui_context.ctx(), |ui| {
        ui.add(egui::Slider::new(&mut ui_state.detail_level, 10..=13).text("Detail"));
        ui.horizontal(|ui| {
            ui.label("Latitude: ");
            ui.text_edit_singleline(&mut ui_state.lat);
        });

        ui.horizontal(|ui| {
            ui.label("Longitude: ");
            ui.text_edit_singleline(&mut ui_state.lon);
        });
    });
}
