use bevy::input::mouse::MouseMotion;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_egui::EguiContext;

#[derive(Debug)]
pub struct OrbitCamera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Default for OrbitCamera {
    fn default() -> OrbitCamera {
        OrbitCamera {
            x: 0.,
            y: 0.,
            zoom: 10.,
        }
    }
}

#[derive(Debug)]
pub enum MouseEvents {
    Drag(Vec2),
    Zoom(f32),
}

pub fn emit_mouse_events(
    mut events: EventWriter<MouseEvents>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mouse_button_input: Res<Input<MouseButton>>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion_events.iter() {
        delta += event.delta;
    }
    if mouse_button_input.pressed(MouseButton::Left) {
        events.send(MouseEvents::Drag(delta))
    }

    let mut total = 0.0;
    for event in mouse_wheel_events.iter() {
        total += event.y
            * match event.unit {
                MouseScrollUnit::Line => 1.0,
                MouseScrollUnit::Pixel => 0.1, // ???
            };
    }

    if total != 0.0 {
        events.send(MouseEvents::Zoom(total));
    }
}

pub fn camera_motion_system(
    time: Res<Time>,
    mut events: EventReader<MouseEvents>,
    mut query: Query<(&mut Transform, &mut OrbitCamera)>,
    egui_context: ResMut<EguiContext>,
) {
    if !egui_context.ctx().is_using_pointer() {
        if let Ok((mut camera_transform, mut camera)) = query.single_mut() {
            for event in events.iter() {
                match event {
                    &MouseEvents::Drag(delta) => {
                        camera.x -= delta.x * time.delta_seconds() * 10.;
                        camera.y += delta.y * time.delta_seconds() * 10.;

                        while camera.x > 360. {
                            camera.x -= 360.;
                        }
                        while camera.x < 0. {
                            camera.x += 360.;
                        }

                        while camera.y > 360. {
                            camera.y -= 360.;
                        }
                        while camera.y < 0. {
                            camera.y += 360.;
                        }
                    }
                    &MouseEvents::Zoom(delta) => {
                        camera.zoom = (camera.zoom - delta * time.delta_seconds() * 20.).max(10.);
                    }
                }
            }

            camera_transform.translation = Quat::from_axis_angle(Vec3::Y, camera.x.to_radians())
                * Quat::from_axis_angle(Vec3::Z, camera.y.to_radians())
                * (Vec3::X * camera.zoom);
            camera_transform.look_at(Vec3::ZERO, Vec3::Y);
        }
    }
}
