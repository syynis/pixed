pub mod tiles;

use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    app.edit_schedule(Main, |schedule| {
        schedule.set_build_settings(bevy::ecs::schedule::ScheduleBuildSettings {
            ambiguity_detection: bevy::ecs::schedule::LogLevel::Error,
            ..default()
        });
    });
    #[cfg(not(debug_assertions))]
    let log_plugin = bevy::log::LogPlugin {
        filter: "off".to_string(),
        ..default()
    };
    #[cfg(debug_assertions)]
    let log_plugin = bevy::log::LogPlugin {
        level: bevy::log::Level::DEBUG,
        filter: "info,wgpu_core=error,wgpu_hal=error,game=debug".into(),
    };
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Lost".to_string(),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
            .set(log_plugin),
        bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
        bevy_pancam::PanCamPlugin,
    ))
    .insert_resource(ClearColor(Color::ANTIQUE_WHITE))
    .insert_resource(Msaa::Off)
    .add_systems(Startup, setup);

    app.run();
}

#[derive(Component, Default)]
struct GameCamera;
fn setup(mut cmds: Commands) {
    cmds.spawn((
        Camera2dBundle::default(),
        #[cfg(debug_assertions)]
        bevy_pancam::PanCam {
            grab_buttons: vec![MouseButton::Middle],
            enabled: true,
            ..default()
        },
        GameCamera,
    ));
}
