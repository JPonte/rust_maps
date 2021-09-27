use bevy::{
    prelude::*,
    render::camera::*,
    tasks::{AsyncComputeTaskPool, Task},
};
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
        .insert_resource(ClearColor(Color::rgb(0.3, 0.56, 0.83)))
        .add_event::<MouseEvents>()
        .add_startup_system(query_stuff.system())
        .add_startup_system(setup.system())
        .add_system(emit_mouse_events.system())
        .add_system(camera_motion_system.system())
        .add_system(handle_tasks.system())
        .add_system(on_camera_updated.system())
        .add_system(on_user_position_updated.system())
        .run();
}

fn query_stuff(mut commands: Commands, thread_pool: Res<AsyncComputeTaskPool>) {
    let task = thread_pool.spawn(async move {
        let lat = 36.253128;
        let lon = -112.521346;
        let z = 13;

        let (x, y) = deg2num(lat, lon, z);

        let topo_filename =
            async_compat::Compat::new(async { get_arcgis_topo_tile(x, y, z).await })
                .await
                .unwrap_or("".to_string());

        let image_filename =
            async_compat::Compat::new(async { get_arcgis_image_tile(x, y, z).await })
                .await
                .unwrap_or("".to_string());

        (topo_filename, image_filename)
    });
    commands.spawn().insert(task);
}

fn handle_tasks(
    mut commands: Commands,
    mut query_tasks: Query<(Entity, &mut Task<(String, String)>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    current_tiles: Query<Entity, With<TerrainTile>>
) {
    for (entity, mut task) in query_tasks.iter_mut() {
        if let Some((topo_filename, image_filename)) =
            future::block_on(future::poll_once(&mut *task))
        {

            for tile in current_tiles.iter() {
                commands.entity(tile).despawn();
            }

            setup_terrain(
                &mut commands,
                &mut meshes,
                &mut materials,
                &asset_server,
                &image_filename[7..],
                topo_filename.as_str(),
            );

            commands.entity(entity).remove::<Task<(String, String)>>();
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
        lat: 36.253128,
        lon: -112.521346,
        zoom: 13,
    });
}

fn on_camera_updated(
    camera_query: Query<&OrbitCamera, Changed<OrbitCamera>>,
    mut user_position_query: Query<&mut UserPosition>,
) {
    for camera in camera_query.iter() {
        for mut user_position in user_position_query.iter_mut() {
            let new_zoom = (14. - (camera.zoom / 50.)) as u32;
            if user_position.zoom != new_zoom {
                user_position.zoom = new_zoom;
            }
        }
    }
}

fn on_user_position_updated(
    query: Query<&UserPosition, Changed<UserPosition>>,
    mut commands: Commands,
    thread_pool: Res<AsyncComputeTaskPool>,
) {
    if let Ok(current_pos) = query.single() {
        
        let user_pos = current_pos.clone();
        
        let task = thread_pool.spawn(async move {
            println!("{:?}", user_pos);

            let (x, y) = deg2num(user_pos.lat, user_pos.lon, user_pos.zoom);

            let topo_filename = async_compat::Compat::new(async {
                get_arcgis_topo_tile(x, y, user_pos.zoom).await
            })
            .await
            .unwrap_or("".to_string());

            let image_filename = async_compat::Compat::new(async {
                get_arcgis_image_tile(x, y, user_pos.zoom).await
            })
            .await
            .unwrap_or("".to_string());

            (topo_filename, image_filename)
        });
        commands.spawn().insert(task);
    }
}
