use core::f32::consts::FRAC_PI_2;

use bevy::color::palettes;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy_motiongfx::BevyMotionGfxPlugin;
use bevy_motiongfx::prelude::*;
use bevy_motiongfx::world::TimelineComplete;

use crate::pipelines_ready::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            BevyMotionGfxPlugin,
            PipelinesReadyPlugin,
        ))
        .add_systems(Startup, (setup, spawn_timeline))
        .add_systems(OnEnter(PipelineState::Ready), start_recording)
        .add_systems(
            Update,
            screenshot.run_if(in_state(PipelineState::Ready)),
        )
        .run();
}

fn screenshot(
    mut commands: Commands,
    q_player: Query<&RecordPlayer, Without<TimelineComplete>>,
) {
    let Ok(player) = q_player.single() else {
        return;
    };

    if !player.is_playing {
        return;
    }

    commands.spawn(Screenshot::primary_window()).observe(
        save_to_disk(format!(
            "frames/frame_{:05}.png",
            player.curr_frame
        )),
    );
}

fn start_recording(mut q_player: Query<&mut RecordPlayer>) {
    let Ok(mut player) = q_player.single_mut() else {
        return;
    };

    player.set_playing(true);
}

// TODO: Quit on last frame captured.
// fn check_final_frame(captured: On<ScreenshotCaptured>) {}

fn spawn_timeline(
    mut commands: Commands,
    mut motiongfx: ResMut<MotionGfxWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn cube.
    let mesh = meshes.add(Cuboid::default());
    let mat = materials.add(StandardMaterial {
        base_color: palettes::tailwind::LIME_200.into(),
        ..default()
    });

    let cube = commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_xyz(-5.0, 0.0, 0.0),
        ))
        .id();

    // Build the timeline.
    let mut b = TimelineBuilder::new();

    let track = b
        .act(cube, field!(<Transform>::translation), |x| {
            x + Vec3::ZERO.with_x(10.0).with_z(1.0)
        })
        .with_interp(|start, end, t| arc_lerp_3d(*start, *end, t))
        .with_ease(ease::cubic::ease_in_out)
        .play(1.0)
        .compile();

    b.add_tracks(track);

    commands.spawn((
        motiongfx.add_timeline(b.compile()),
        RecordPlayer::new(30),
    ));
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
        Camera3d::default(),
        // Top down view.
        Transform::from_xyz(0.0, 18.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// TODO: Optimize this.
pub fn arc_lerp_3d(start: Vec3, end: Vec3, t: f32) -> Vec3 {
    let center = (start + end) * 0.5;

    let start_dir = Dir3::new(start - center);
    let end_dir = Dir3::new(end - center);

    let (Ok(start_dir), Ok(end_dir)) = (start_dir, end_dir) else {
        // Revert to linear interpolation.
        return start.lerp(end, t);
    };

    let target_dir = start_dir.slerp(end_dir, t);

    center + target_dir.as_vec3() * (center - start).length()
}

mod pipelines_ready {
    use bevy::{
        prelude::*,
        render::{render_resource::*, *},
    };

    #[derive(
        States,
        Default,
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
    )]
    pub enum PipelineState {
        #[default]
        Loading,
        Ready,
    }

    pub struct PipelinesReadyPlugin;
    impl Plugin for PipelinesReadyPlugin {
        fn build(&self, app: &mut App) {
            app.init_state::<PipelineState>();

            // In order to gain access to the pipelines status, we have to
            // go into the `RenderApp`, grab the resource from the main App
            // and then update the pipelines status from there.
            // Writing between these Apps can only be done through the
            // `ExtractSchedule`.
            app.sub_app_mut(RenderApp)
                .add_systems(ExtractSchedule, update_pipelines_ready);
        }
    }

    fn update_pipelines_ready(
        mut main_world: ResMut<MainWorld>,
        pipelines: Res<PipelineCache>,
    ) {
        let curr_state =
            main_world.resource::<State<PipelineState>>();
        if *curr_state.get() == PipelineState::Ready {
            return;
        }

        let mut state =
            main_world.resource_mut::<NextState<PipelineState>>();

        // If there are pipelines cerated and all of them are already
        // initialized.
        if pipelines.pipelines().count() > 0
            && pipelines.waiting_pipelines().count() == 0
        {
            state.set(PipelineState::Ready);
        }
    }
}
