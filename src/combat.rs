use bevy::{prelude::*, render::camera::Camera2d};
use bevy_inspector_egui::Inspectable;

use crate::{
    ascii::{
        spawn_ascii_sprite, spawn_ascii_text, spawn_nine_slice, AsciiSheet, NineSlice,
        NineSliceIndices,
    },
    fadeout::create_fadeout,
    graphics::{spawn_enemy_sprite, CharacterSheet, VfxSheet},
    player::{Player, self},
    GameState, RESOLUTION, TILE_SIZE,
};

#[derive(Component)]
pub struct Enemy {
    enemy_type: EnemyType,
}

pub const MENU_COUNT: isize = 3;

#[derive(Component, PartialEq, Eq, Clone, Copy)]
pub enum CombatMenuOption {
    Attack,
    MagicAttack,
    Run,
}

#[derive(Component)]
pub struct DespawnTimer(Timer);

#[derive(Component)]
pub struct CombatText;

#[derive(Component)]
pub struct CombatManaText;

pub struct CombatPlugin;

pub struct FightEvent {
    target: Entity,
    attack_type: AttackType,
    damage_amount: isize,
    next_state: CombatState,
}

#[derive(Component, Inspectable)]
pub struct CombatStats {
    pub health: isize,
    pub max_health: isize,
    pub mana: isize,
    pub max_mana: isize,
    pub attack: isize,
    pub defense: isize,
}

#[derive(Clone, Copy)]
pub enum EnemyType {
    Bat,
    Ghost,
}

#[derive(Clone, Copy)]
pub enum AttackType {
    Standard,
    MagicGeneric,
    MagicFire,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct CombatMenuSelection {
    selected: CombatMenuOption,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum CombatState {
    PlayerTurn,
    PlayerAttack,
    EnemyTurn(bool),
    EnemyAttack,
    Reward,
    Exiting,
}

pub struct AttackEffects {
    timer: Timer,
    flash_speed: f32,
    screen_shake_amount: f32,
    current_shake: f32,
}

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FightEvent>()
            .add_state(CombatState::PlayerTurn)
            .insert_resource(AttackEffects {
                timer: Timer::from_seconds(0.7, true),
                flash_speed: 0.1,
                screen_shake_amount: 0.1,
                current_shake: 0.0,
            })
            .insert_resource(CombatMenuSelection {
                selected: CombatMenuOption::Attack,
            })
            .add_system(despawn_system)
            .add_system_set(
                SystemSet::on_update(CombatState::EnemyTurn(false)).with_system(process_enemy_turn),
            )
            .add_system_set(
                SystemSet::on_update(GameState::Combat)
                    .with_system(combat_input)
                    .with_system(combat_damage_calc)
                    .with_system(highlight_combat_buttons)
                    .with_system(combat_camera),
            )
            .add_system_set(
                SystemSet::on_enter(GameState::Combat)
                    .with_system(set_starting_state)
                    .with_system(spawn_enemy)
                    .with_system(spawn_player_stats_texts)
                    .with_system(spawn_combat_menu),
            )
            .add_system_set(
                SystemSet::on_exit(GameState::Combat)
                    .with_system(despawn_all_combat_text)
                    .with_system(despawn_menu)
                    .with_system(despawn_enemy),
            )
            .add_system_set(
                SystemSet::on_enter(CombatState::PlayerAttack)
                    .with_system(handle_initial_attack_effects),
            )
            .add_system_set(
                SystemSet::on_update(CombatState::PlayerAttack).with_system(handle_attack_effects),
            )
            .add_system_set(
                SystemSet::on_enter(CombatState::Reward)
                    .with_system(give_reward)
                    .with_system(despawn_enemy),
            )
            .add_system_set(
                SystemSet::on_update(CombatState::Reward).with_system(handle_accepting_reward),
            )
            .add_system_set(
                SystemSet::on_update(CombatState::EnemyAttack).with_system(handle_attack_effects),
            );
    }
}

fn spawn_player_stats_texts(
    mut commands: Commands,
    ascii: Res<AsciiSheet>,
    player_query: Query<(Entity, &CombatStats, &Transform), With<Player>>,
) {
    let (player, stats, transform) = player_query.single();

    // health
    let health_text_string = format!("Health: {}", stats.health);
    let health_text = spawn_ascii_text(
        &mut commands,
        &ascii,
        &health_text_string,
        Vec3::new(-RESOLUTION + TILE_SIZE, -1.0 + TILE_SIZE, 0.0) - transform.translation,
    );
    commands
        .entity(health_text)
        .insert(CombatText)
        .insert(Name::new("health_text"));
    commands.entity(player).add_child(health_text);

    // mana
    let mana_text_string = format!("Mana: {}", stats.mana);
    let mana_text = spawn_ascii_text(
        &mut commands,
        &ascii,
        &mana_text_string,
        Vec3::new(-RESOLUTION + TILE_SIZE, -0.9 + TILE_SIZE, 0.0) - transform.translation,
    );
    commands
        .entity(mana_text)
        .insert(CombatManaText)
        .insert(Name::new("mana_text"));
    commands.entity(player).add_child(mana_text);
}

fn handle_initial_attack_effects(
    mut commands: Commands,
    ascii: Res<AsciiSheet>,
    vfx_sheet: Res<VfxSheet>,
    mut enemy_graphics_query: Query<&Transform, With<Enemy>>,
    mut event_reader: EventReader<FightEvent>,
) {
    let enemy_transform = enemy_graphics_query.iter_mut().next().unwrap();
    let mut vfx_index = 0;
    for event in event_reader.iter() {
        vfx_index = match event.attack_type {
            AttackType::Standard => vfx_sheet.slash,
            AttackType::MagicGeneric => vfx_sheet.magic,
            AttackType::MagicFire => vfx_sheet.slash,
        }
    }

    let attack_vfx = spawn_ascii_sprite(
        &mut commands,
        &ascii,
        vfx_index,
        Color::rgb(0.9, 0.9, 0.9),
        Vec3::new(
            enemy_transform.translation.x,
            enemy_transform.translation.y,
            150.0,
        ),
        Vec3::splat(6.0),
    );

    commands
        .entity(attack_vfx)
        .insert(DespawnTimer(Timer::from_seconds(0.3, false)));
}

fn handle_attack_effects(
    mut attack_fx: ResMut<AttackEffects>,
    time: Res<Time>,
    mut enemy_graphics_query: Query<&mut Visibility, With<Enemy>>,
    mut state: ResMut<State<CombatState>>,
) {
    attack_fx.timer.tick(time.delta());
    let mut enemy_sprite = enemy_graphics_query.iter_mut().next().unwrap();

    if state.current() == &CombatState::PlayerAttack {
        if attack_fx.timer.elapsed_secs() % attack_fx.flash_speed > attack_fx.flash_speed / 2.0 {
            enemy_sprite.is_visible = false;
        } else {
            enemy_sprite.is_visible = true;
        }
    } else {
        attack_fx.current_shake = attack_fx.screen_shake_amount
            * f32::sin(attack_fx.timer.percent() * 2.0 * std::f32::consts::PI);
    }

    if attack_fx.timer.just_finished() {
        enemy_sprite.is_visible = true;
        if state.current() == &CombatState::PlayerAttack {
            state.set(CombatState::EnemyTurn(false)).unwrap();
        } else {
            state.set(CombatState::PlayerTurn).unwrap();
        }
    }
}

fn set_starting_state(mut combat_state: ResMut<State<CombatState>>) {
    // TODO speed and turn calculations
    // throw away error if it occurs
    let _ = combat_state.set(CombatState::PlayerTurn);
}

fn process_enemy_turn(
    mut fight_event: EventWriter<FightEvent>,
    mut combat_state: ResMut<State<CombatState>>,
    enemy_query: Query<&CombatStats, With<Enemy>>,
    player_query: Query<Entity, With<Player>>,
) {
    let player_ent = player_query.single();
    // TODO support multiple enemies
    let enemy_stats = enemy_query.iter().next().unwrap();

    fight_event.send(FightEvent {
        target: player_ent,
        attack_type: AttackType::Standard,
        damage_amount: enemy_stats.attack,
        next_state: CombatState::EnemyAttack,
    });
    combat_state.set(CombatState::EnemyTurn(true)).unwrap();
}

fn handle_accepting_reward(
    mut commands: Commands,
    ascii: Res<AsciiSheet>,
    keyboard: Res<Input<KeyCode>>,
    mut combat_state: ResMut<State<CombatState>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        combat_state.set(CombatState::Exiting).unwrap();
        create_fadeout(&mut commands, None, &ascii);
    }
}

fn give_reward(
    mut commands: Commands,
    ascii: Res<AsciiSheet>,
    mut player_query: Query<(&mut Player, &mut CombatStats)>,
    enemy_query: Query<&Enemy>,
) {
    let exp_reward = match enemy_query.single().enemy_type {
        EnemyType::Bat => 10,
        EnemyType::Ghost => 30,
    };
    let reward_text = format!("Earned {} exp", exp_reward);
    let text = spawn_ascii_text(
        &mut commands,
        &ascii,
        &reward_text,
        Vec3::new(-((reward_text.len() / 2) as f32 * TILE_SIZE), 0.0, 0.0),
    );

    commands.entity(text).insert(CombatText);
    let (mut player, mut stats) = player_query.single_mut();
    if player.give_exp(exp_reward, &mut stats) {
        let level_text = "Level up!";
        let text = spawn_ascii_text(
            &mut commands,
            &ascii,
            level_text,
            Vec3::new(
                -((level_text.len() / 2) as f32 * TILE_SIZE),
                -1.5 * TILE_SIZE,
                0.0,
            ),
        );
        commands.entity(text).insert(CombatText);
    }
}

fn despawn_menu(mut commands: Commands, button_query: Query<Entity, With<CombatMenuOption>>) {
    for button in button_query.iter() {
        commands.entity(button).despawn_recursive();
    }
}

fn despawn_all_combat_text(mut commands: Commands, text_query: Query<Entity, Or<(With<CombatText>, With<CombatManaText>)>>) {
    for entity in text_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_enemy(mut commands: Commands, ascii: Res<AsciiSheet>, characters: Res<CharacterSheet>) {
    let enemy_type = match rand::random::<f32>() {
        x if x < 0.5 => EnemyType::Bat,
        _ => EnemyType::Ghost,
    };
    let stats = match enemy_type {
        EnemyType::Bat => CombatStats {
            health: 3,
            max_health: 3,
            mana: 0,
            max_mana: 0,
            attack: 2,
            defense: 1,
        },
        EnemyType::Ghost => CombatStats {
            health: 5,
            max_health: 5,
            mana: 0,
            max_mana: 0,
            attack: 3,
            defense: 2,
        },
    };

    let health_text = spawn_ascii_text(
        &mut commands,
        &ascii,
        &format!("Health: {}", stats.health as usize),
        //relative to enemy pos
        Vec3::new(-4.5 * TILE_SIZE, 0.5, 100.0),
    );
    commands.entity(health_text).insert(CombatText);
    let sprite = spawn_enemy_sprite(
        &mut commands,
        &characters,
        Vec3::new(0.0, 0.3, 100.0),
        enemy_type,
    );
    commands
        .entity(sprite)
        .insert(Enemy { enemy_type })
        .insert(stats)
        .insert(Name::new("Bat"))
        .add_child(health_text);
}

fn despawn_enemy(mut commands: Commands, enemy_query: Query<Entity, With<Enemy>>) {
    for entity in enemy_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn highlight_combat_buttons(
    menu_state: Res<CombatMenuSelection>,
    button_query: Query<(&Children, &CombatMenuOption)>,
    nine_slice_query: Query<&Children, With<NineSlice>>,
    mut sprites_query: Query<&mut TextureAtlasSprite>,
) {
    for (button_children, button_id) in button_query.iter() {
        for button_child in button_children.iter() {
            // Get nine slice children from each button
            if let Ok(nine_slice_children) = nine_slice_query.get(*button_child) {
                for nine_slice_child in nine_slice_children.iter() {
                    // If the nine slice child is a sprite color it
                    if let Ok(mut sprite) = sprites_query.get_mut(*nine_slice_child) {
                        if menu_state.selected == *button_id {
                            sprite.color = Color::RED;
                        } else {
                            sprite.color = Color::WHITE;
                        }
                    }
                }
            }
        }
    }
}

fn spawn_combat_button(
    commands: &mut Commands,
    ascii: &AsciiSheet,
    indices: &NineSliceIndices,
    translation: Vec3,
    text: &str,
    id: CombatMenuOption,
    size: Vec2,
) -> Entity {
    let nine_slice = spawn_nine_slice(commands, ascii, indices, size.x, size.y);

    let x_offset = (-size.x / 2.0 + 1.5) * TILE_SIZE;
    let text = spawn_ascii_text(commands, ascii, text, Vec3::new(x_offset, 0.0, 0.0));

    commands
        .spawn()
        .insert(Transform {
            translation: translation,
            ..Default::default()
        })
        .insert(GlobalTransform::default())
        .insert(Name::new("Button"))
        .insert(id)
        .add_child(nine_slice)
        .add_child(text)
        .id()
}

fn spawn_combat_menu(
    mut commands: Commands,
    ascii: Res<AsciiSheet>,
    nine_slice_indices: Res<NineSliceIndices>,
) {
    // TODO rework spawning combat menu in loop

    let box_height = 3.0;
    let box_center_y = -1.0 + box_height * TILE_SIZE / 2.0;

    let run_text = "Run";
    let run_width = (run_text.len() + 2) as f32;
    let run_center_x = RESOLUTION - (run_width * TILE_SIZE) / 2.0;
    spawn_combat_button(
        &mut commands,
        &ascii,
        &nine_slice_indices,
        Vec3::new(run_center_x, box_center_y, 100.0),
        run_text,
        CombatMenuOption::Run,
        Vec2::new(run_width, box_height),
    );

    let magic_text = "Magic";
    let magic_width = (magic_text.len() + 2) as f32;
    let magic_center_x = RESOLUTION - (run_width * TILE_SIZE) - (magic_width * TILE_SIZE / 2.0);
    spawn_combat_button(
        &mut commands,
        &ascii,
        &nine_slice_indices,
        Vec3::new(magic_center_x, box_center_y, 100.0),
        magic_text,
        CombatMenuOption::MagicAttack,
        Vec2::new(magic_width, box_height),
    );

    let attack_text = "Attack";
    let attack_width = (attack_text.len() + 2) as f32;
    let attack_center_x = RESOLUTION
        - (run_width * TILE_SIZE)
        - (magic_width * TILE_SIZE)
        - (attack_width * TILE_SIZE / 2.0);
    spawn_combat_button(
        &mut commands,
        &ascii,
        &nine_slice_indices,
        Vec3::new(attack_center_x, box_center_y, 100.0),
        attack_text,
        CombatMenuOption::Attack,
        Vec2::new(attack_width, box_height),
    );
}

fn combat_damage_calc(
    mut commands: Commands,
    mut fight_event: EventReader<FightEvent>,
    //Not necssacarily enemy
    mut enemy_query: Query<(&Children, &mut CombatStats)>,
    ascii: Res<AsciiSheet>,
    text_query: Query<&Transform, With<CombatText>>,
    mut combat_state: ResMut<State<CombatState>>,
) {
    if let Some(fight_event) = fight_event.iter().next() {
        //Get target stats and children
        let (target_children, mut stats) = enemy_query
            .get_mut(fight_event.target)
            .expect("Fighting enemy without stats");

        //Damage calc
        stats.health = std::cmp::max(
            stats.health - (fight_event.damage_amount - stats.defense),
            0,
        );

        //Update health
        for child in target_children.iter() {
            //See if this child is the health text
            if let Ok(transform) = text_query.get(*child) {
                //Delete old text
                commands.entity(*child).despawn_recursive();
                //Create new text
                let new_health = spawn_ascii_text(
                    &mut commands,
                    &ascii,
                    &format!("Health: {}", stats.health as usize),
                    //relative to enemy pos
                    transform.translation,
                );
                commands.entity(new_health).insert(CombatText);
                commands.entity(fight_event.target).add_child(new_health);
            }
        }

        //Kill enemy if dead
        //TODO support multiple enemies
        if stats.health == 0 {
            combat_state.set(CombatState::Reward).unwrap();
        } else {
            combat_state.set(fight_event.next_state).unwrap();
        }
    }
}

fn combat_input(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    mut fight_event_writer: EventWriter<FightEvent>,
    mut player_query: Query<(&mut CombatStats, &Children, Entity), With<Player>>,
    enemy_query: Query<Entity, With<Enemy>>,
    mut menu_state: ResMut<CombatMenuSelection>,
    ascii: Res<AsciiSheet>,
    combat_state: Res<State<CombatState>>,
    mana_text: Query<&Transform, With<CombatManaText>>,
) {
    if combat_state.current() != &CombatState::PlayerTurn {
        return;
    }

    let mut new_selection = menu_state.selected as isize;

    if keyboard.just_pressed(KeyCode::A) {
        new_selection -= 1;
    }
    if keyboard.just_pressed(KeyCode::D) {
        new_selection += 1;
    }
    new_selection = (new_selection + MENU_COUNT) % MENU_COUNT;

    menu_state.selected = match new_selection {
        0 => CombatMenuOption::Attack,
        1 => CombatMenuOption::MagicAttack,
        2 => CombatMenuOption::Run,
        _ => unreachable!("Bad menu selection"),
    };

    if keyboard.just_pressed(KeyCode::Return) {
        match menu_state.selected {
            CombatMenuOption::Attack => {
                let (player_stats, player_children, player_entity) = player_query.single();
                // TODO handle multiple enemies and enemy selection
                let target = enemy_query.iter().next().unwrap();

                fight_event_writer.send(FightEvent {
                    target: target,
                    attack_type: AttackType::Standard,
                    damage_amount: player_stats.attack,
                    next_state: CombatState::PlayerAttack,
                });
            }
            CombatMenuOption::MagicAttack => {
                let (mut player_stats, player_children, player_entity) = player_query.single_mut();
                let target = enemy_query.iter().next().unwrap();

                if player_stats.mana > 0 {
                    player_stats.mana -= 1;

                    //Update mana
                    for child in player_children.iter() {
                        //See if this child is the health text
                        if let Ok(transform) = mana_text.get(*child) {
                            //Delete old text
                            commands.entity(*child).despawn_recursive();
                            //Create new text
                            let new_mana_text = spawn_ascii_text(
                                &mut commands,
                                &ascii,
                                &format!("Mana: {}", player_stats.mana as usize),
                                //relative to enemy pos
                                transform.translation,
                            );
                            commands.entity(new_mana_text).insert(CombatManaText);
                            commands.entity(player_entity).add_child(new_mana_text);
                        }
                    }

                    fight_event_writer.send(FightEvent {
                        target: target,
                        attack_type: AttackType::MagicGeneric,
                        damage_amount: 4,
                        next_state: CombatState::PlayerAttack,
                    });
                }
            }
            CombatMenuOption::Run => {
                create_fadeout(&mut commands, None, &ascii);
            }
        }
    }
}

fn combat_camera(
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    attack_fx: Res<AttackEffects>,
) {
    let mut camera_transform = camera_query.single_mut();
    camera_transform.translation.x = attack_fx.current_shake;
    camera_transform.translation.y = 0.0;
}

fn despawn_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut DespawnTimer)>,
) {
    for (entity, mut despawn_timer) in query.iter_mut() {
        despawn_timer.0.tick(time.delta());

        if despawn_timer.0.finished() {
            commands.entity(entity).despawn();
        }
    }
}
