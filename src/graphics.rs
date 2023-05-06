use bevy::animation;
use::bevy::prelude::*;

pub struct GraphicsPlugin;

pub struct CharacterSheet {
    pub handle: Handle<TextureAtlas>,
    pub player_up: [usize; 3],
    pub player_down: [usize; 3],
    pub player_left: [usize; 3],
    pub player_right: [usize; 3],
    pub bat_frames: [usize; 3]
}

pub enum FacingDirection {
    Up,
    Down,
    Left,
    Right
}

#[derive(Component)]
pub struct PlayerGraphics {
    pub facing: FacingDirection,
}

#[derive(Component)]
pub struct FrameAnimation {
    pub timer: Timer,
    pub frames: Vec<usize>,
    pub current_frame: usize
}

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system_to_stage(StartupStage::PreStartup, Self::load_graphics)
            .add_system(Self::frame_animation)
            .add_system(Self::update_player_graphics);
    }
}

pub fn spawn_bat_sprite(
    commands: &mut Commands,
    characters: &CharacterSheet,
    translation: Vec3
) -> Entity {
    let mut sprite = TextureAtlasSprite::new(characters.bat_frames[0]);
    sprite.custom_size = Some(Vec2::splat(0.5));

    commands.spawn_bundle(SpriteSheetBundle {
        sprite: sprite,
        texture_atlas: characters.handle.clone(),
        transform: Transform {
            translation: translation,
            ..Default::default()
        },
        ..Default::default()
    })
    .insert(FrameAnimation {
        timer: Timer::from_seconds(0.2, true),
        frames: characters.bat_frames.to_vec(),
        current_frame: 0
    })
    .id()
}

impl GraphicsPlugin {
    fn load_graphics(
        mut commands: Commands,
        assets: Res<AssetServer>,
        mut texture_atlases: ResMut<Assets<TextureAtlas>>
    ) {
        let image = assets.load("characters.png");
        let atlas = TextureAtlas::from_grid_with_padding(
            image, Vec2::splat(16.0), 12, 8, Vec2::splat(2.0)
        );
        let atlas_handle = texture_atlases.add(atlas);

        let columns = 12;

        commands.insert_resource(CharacterSheet {
            handle: atlas_handle,
            player_down: [columns * 0 + 3, columns * 0 + 4, columns * 0 + 5],
            player_left: [columns * 1 + 3, columns * 1 + 4, columns * 1 + 5],
            player_right: [columns * 2 + 3, columns * 2 + 4, columns * 2 + 5],
            player_up: [columns * 3 + 3, columns * 3 + 4, columns * 3 + 5],
            bat_frames: [columns * 4 + 3, columns * 4 + 4, columns * 4 + 5],
        })
    }

    fn update_player_graphics(
        mut sprites_query: Query<(&PlayerGraphics, &mut FrameAnimation), Changed<PlayerGraphics>>,
        characters: Res<CharacterSheet>
    ) {
        for (graphics, mut animation) in sprites_query.iter_mut() {
            animation.frames = match graphics.facing {
                FacingDirection::Up => characters.player_up.to_vec(),
                FacingDirection::Down => characters.player_down.to_vec(),
                FacingDirection::Left => characters.player_left.to_vec(),
                FacingDirection::Right => characters.player_right.to_vec(),
            }
        }
    }

    fn frame_animation(
        mut sprites_query: Query<(&mut TextureAtlasSprite, &mut FrameAnimation)>,
        time: Res<Time>
    ) {
        for (mut sprite, mut animation) in sprites_query.iter_mut() {
            animation.timer.tick(time.delta());
            if animation.timer.just_finished() {
                animation.current_frame = (animation.current_frame + 1) % animation.frames.len();
                sprite.index = animation.frames[animation.current_frame];
            }
        }
    }
}