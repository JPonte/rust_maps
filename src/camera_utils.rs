use bevy::input::mouse::MouseMotion;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::{prelude::*, render::camera::*};

pub struct OrbitCamera;

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
    mut query: Query<(&mut Transform, &mut Camera), With<OrbitCamera>>,
) {
    if let Ok((mut camera_transform, camera)) = query.single_mut() {
        for event in events.iter() {
            match event {
                &MouseEvents::Drag(delta) => {
                    let rot_y =
                        Quat::from_axis_angle(Vec3::Y, delta.x * time.delta_seconds() * -0.5);
                    let rot_x =
                        Quat::from_axis_angle(Vec3::X, delta.y * time.delta_seconds() * 0.5);

                    camera_transform.translation = rot_x * rot_y * camera_transform.translation;

                    camera_transform.look_at(Vec3::ZERO, Vec3::Y);
                }
                &MouseEvents::Zoom(delta) => {
                    let delta_vec = camera_transform.translation.normalize_or_zero()
                        * delta
                        * time.delta_seconds()
                        * -20.;
                    camera_transform.translation += delta_vec;
                }
            }
        }
    }
}
