use azalea_registry::builtin::BlockKind;
use glam::Vec3;

use super::aabb::Aabb;
use crate::world::chunk::ChunkStore;

fn has_collision(state: azalea_block::BlockState) -> bool {
    if state.is_air() {
        return false;
    }
    let kind: BlockKind = state.into();
    !matches!(
        kind,
        // Flowers
        BlockKind::Dandelion
            | BlockKind::Poppy
            | BlockKind::BlueOrchid
            | BlockKind::Allium
            | BlockKind::AzureBluet
            | BlockKind::RedTulip
            | BlockKind::OrangeTulip
            | BlockKind::WhiteTulip
            | BlockKind::PinkTulip
            | BlockKind::OxeyeDaisy
            | BlockKind::Cornflower
            | BlockKind::LilyOfTheValley
            | BlockKind::WitherRose
            | BlockKind::Torchflower
            | BlockKind::CactusFlower
            | BlockKind::OpenEyeblossom
            | BlockKind::ClosedEyeblossom
            | BlockKind::PinkPetals
            | BlockKind::Wildflowers
            | BlockKind::SporeBlossom
            // Tall flowers
            | BlockKind::Sunflower
            | BlockKind::Lilac
            | BlockKind::RoseBush
            | BlockKind::Peony
            | BlockKind::PitcherPlant
            // Grass & ferns
            | BlockKind::ShortGrass
            | BlockKind::TallGrass
            | BlockKind::Fern
            | BlockKind::LargeFern
            | BlockKind::DeadBush
            | BlockKind::ShortDryGrass
            | BlockKind::TallDryGrass
            // Saplings
            | BlockKind::OakSapling
            | BlockKind::SpruceSapling
            | BlockKind::BirchSapling
            | BlockKind::JungleSapling
            | BlockKind::AcaciaSapling
            | BlockKind::DarkOakSapling
            | BlockKind::CherrySapling
            | BlockKind::PaleOakSapling
            | BlockKind::BambooSapling
            | BlockKind::MangrovePropagule
            // Crops
            | BlockKind::Wheat
            | BlockKind::Carrots
            | BlockKind::Potatoes
            | BlockKind::Beetroots
            | BlockKind::TorchflowerCrop
            | BlockKind::PitcherCrop
            | BlockKind::MelonStem
            | BlockKind::PumpkinStem
            | BlockKind::AttachedMelonStem
            | BlockKind::AttachedPumpkinStem
            | BlockKind::NetherWart
            | BlockKind::SweetBerryBush
            | BlockKind::SugarCane
            // Underwater plants
            | BlockKind::Seagrass
            | BlockKind::TallSeagrass
            | BlockKind::Kelp
            | BlockKind::KelpPlant
            // Corals
            | BlockKind::BrainCoral
            | BlockKind::BrainCoralFan
            | BlockKind::BrainCoralWallFan
            | BlockKind::BubbleCoral
            | BlockKind::BubbleCoralFan
            | BlockKind::BubbleCoralWallFan
            | BlockKind::FireCoral
            | BlockKind::FireCoralFan
            | BlockKind::FireCoralWallFan
            | BlockKind::HornCoral
            | BlockKind::HornCoralFan
            | BlockKind::HornCoralWallFan
            | BlockKind::TubeCoral
            | BlockKind::TubeCoralFan
            | BlockKind::TubeCoralWallFan
            | BlockKind::DeadBrainCoral
            | BlockKind::DeadBrainCoralFan
            | BlockKind::DeadBrainCoralWallFan
            | BlockKind::DeadBubbleCoral
            | BlockKind::DeadBubbleCoralFan
            | BlockKind::DeadBubbleCoralWallFan
            | BlockKind::DeadFireCoral
            | BlockKind::DeadFireCoralFan
            | BlockKind::DeadFireCoralWallFan
            | BlockKind::DeadHornCoral
            | BlockKind::DeadHornCoralFan
            | BlockKind::DeadHornCoralWallFan
            | BlockKind::DeadTubeCoral
            | BlockKind::DeadTubeCoralFan
            | BlockKind::DeadTubeCoralWallFan
            // Mushrooms
            | BlockKind::BrownMushroom
            | BlockKind::RedMushroom
            // Torches
            | BlockKind::Torch
            | BlockKind::WallTorch
            | BlockKind::SoulTorch
            | BlockKind::SoulWallTorch
            | BlockKind::RedstoneTorch
            | BlockKind::RedstoneWallTorch
            | BlockKind::CopperTorch
            | BlockKind::CopperWallTorch
            // Redstone
            | BlockKind::RedstoneWire
            // Rails
            | BlockKind::Rail
            | BlockKind::PoweredRail
            | BlockKind::DetectorRail
            | BlockKind::ActivatorRail
            // Signs
            | BlockKind::OakSign
            | BlockKind::SpruceSign
            | BlockKind::BirchSign
            | BlockKind::JungleSign
            | BlockKind::AcaciaSign
            | BlockKind::DarkOakSign
            | BlockKind::CherrySign
            | BlockKind::PaleOakSign
            | BlockKind::MangroveSign
            | BlockKind::BambooSign
            | BlockKind::CrimsonSign
            | BlockKind::WarpedSign
            | BlockKind::OakWallSign
            | BlockKind::SpruceWallSign
            | BlockKind::BirchWallSign
            | BlockKind::JungleWallSign
            | BlockKind::AcaciaWallSign
            | BlockKind::DarkOakWallSign
            | BlockKind::CherryWallSign
            | BlockKind::PaleOakWallSign
            | BlockKind::MangroveWallSign
            | BlockKind::BambooWallSign
            | BlockKind::CrimsonWallSign
            | BlockKind::WarpedWallSign
            | BlockKind::OakHangingSign
            | BlockKind::SpruceHangingSign
            | BlockKind::BirchHangingSign
            | BlockKind::JungleHangingSign
            | BlockKind::AcaciaHangingSign
            | BlockKind::DarkOakHangingSign
            | BlockKind::CherryHangingSign
            | BlockKind::PaleOakHangingSign
            | BlockKind::MangroveHangingSign
            | BlockKind::BambooHangingSign
            | BlockKind::CrimsonHangingSign
            | BlockKind::WarpedHangingSign
            | BlockKind::OakWallHangingSign
            | BlockKind::SpruceWallHangingSign
            | BlockKind::BirchWallHangingSign
            | BlockKind::JungleWallHangingSign
            | BlockKind::AcaciaWallHangingSign
            | BlockKind::DarkOakWallHangingSign
            | BlockKind::CherryWallHangingSign
            | BlockKind::PaleOakWallHangingSign
            | BlockKind::MangroveWallHangingSign
            | BlockKind::BambooWallHangingSign
            | BlockKind::CrimsonWallHangingSign
            | BlockKind::WarpedWallHangingSign
            // Buttons
            | BlockKind::StoneButton
            | BlockKind::OakButton
            | BlockKind::SpruceButton
            | BlockKind::BirchButton
            | BlockKind::JungleButton
            | BlockKind::AcaciaButton
            | BlockKind::DarkOakButton
            | BlockKind::CherryButton
            | BlockKind::PaleOakButton
            | BlockKind::MangroveButton
            | BlockKind::BambooButton
            | BlockKind::CrimsonButton
            | BlockKind::WarpedButton
            | BlockKind::PolishedBlackstoneButton
            // Vines & climbing plants
            | BlockKind::Vine
            | BlockKind::CaveVines
            | BlockKind::CaveVinesPlant
            | BlockKind::WeepingVines
            | BlockKind::WeepingVinesPlant
            | BlockKind::TwistingVines
            | BlockKind::TwistingVinesPlant
            | BlockKind::GlowLichen
            | BlockKind::SculkVein
            | BlockKind::ResinClump
            // Nether plants
            | BlockKind::CrimsonRoots
            | BlockKind::WarpedRoots
            | BlockKind::NetherSprouts
            | BlockKind::HangingRoots
            | BlockKind::WarpedFungus
            | BlockKind::CrimsonFungus
            // Fire
            | BlockKind::Fire
            | BlockKind::SoulFire
            // Pale garden
            | BlockKind::PaleHangingMoss
            | BlockKind::PaleMossCarpet
            // Other passable blocks
            | BlockKind::Cobweb
            | BlockKind::Water
            | BlockKind::Lava
            | BlockKind::Light
            | BlockKind::StructureVoid
            | BlockKind::VoidAir
            | BlockKind::CaveAir
            | BlockKind::Tripwire
            | BlockKind::TripwireHook
            | BlockKind::Frogspawn
            | BlockKind::LeafLitter
            | BlockKind::FireflyBush
    )
}

pub fn collect_block_aabbs(chunk_store: &ChunkStore, region: &Aabb) -> Vec<Aabb> {
    let mut aabbs = Vec::new();

    let min_x = region.min.x.floor() as i32;
    let min_y = region.min.y.floor() as i32;
    let min_z = region.min.z.floor() as i32;
    let max_x = region.max.x.ceil() as i32;
    let max_y = region.max.y.ceil() as i32;
    let max_z = region.max.z.ceil() as i32;

    for by in min_y..max_y {
        for bz in min_z..max_z {
            for bx in min_x..max_x {
                let state = chunk_store.get_block_state(bx, by, bz);
                if has_collision(state) {
                    aabbs.push(Aabb::new(
                        Vec3::new(bx as f32, by as f32, bz as f32),
                        Vec3::new((bx + 1) as f32, (by + 1) as f32, (bz + 1) as f32),
                    ));
                }
            }
        }
    }

    aabbs
}

fn collide_along_axes(block_aabbs: &[Aabb], player_aabb: Aabb, mut velocity: Vec3) -> (Vec3, bool) {
    let original_y = velocity.y;

    for block in block_aabbs {
        velocity.y = block.clip_y_collide(&player_aabb, velocity.y);
    }
    let mut resolved = player_aabb.offset(Vec3::new(0.0, velocity.y, 0.0));

    let x_first = velocity.x.abs() >= velocity.z.abs();

    if x_first {
        for block in block_aabbs {
            velocity.x = block.clip_x_collide(&resolved, velocity.x);
        }
        resolved = resolved.offset(Vec3::new(velocity.x, 0.0, 0.0));

        for block in block_aabbs {
            velocity.z = block.clip_z_collide(&resolved, velocity.z);
        }
    } else {
        for block in block_aabbs {
            velocity.z = block.clip_z_collide(&resolved, velocity.z);
        }
        resolved = resolved.offset(Vec3::new(0.0, 0.0, velocity.z));

        for block in block_aabbs {
            velocity.x = block.clip_x_collide(&resolved, velocity.x);
        }
    }

    let on_ground = original_y < 0.0 && velocity.y != original_y;

    (velocity, on_ground)
}

pub fn resolve_collision(
    chunk_store: &ChunkStore,
    player_aabb: Aabb,
    velocity: Vec3,
    step_height: f32,
) -> (Vec3, bool) {
    let expanded = player_aabb.expand(velocity);
    let block_aabbs = collect_block_aabbs(chunk_store, &expanded);

    let (resolved, on_ground) = collide_along_axes(&block_aabbs, player_aabb, velocity);

    let horizontal_blocked = resolved.x != velocity.x || resolved.z != velocity.z;
    if step_height > 0.0 && on_ground && horizontal_blocked {
        let step_up = Vec3::new(velocity.x, step_height, velocity.z);
        let step_expanded = player_aabb
            .expand(step_up)
            .expand(Vec3::new(0.0, -step_height, 0.0));
        let step_aabbs = collect_block_aabbs(chunk_store, &step_expanded);

        let mut up_vel = step_height;
        for block in &step_aabbs {
            up_vel = block.clip_y_collide(&player_aabb, up_vel);
        }
        let raised = player_aabb.offset(Vec3::new(0.0, up_vel, 0.0));

        let (step_resolved, _) =
            collide_along_axes(&step_aabbs, raised, Vec3::new(velocity.x, 0.0, velocity.z));

        let after_move = raised.offset(Vec3::new(step_resolved.x, 0.0, step_resolved.z));
        let mut down_vel = -(up_vel - velocity.y);
        for block in &step_aabbs {
            down_vel = block.clip_y_collide(&after_move, down_vel);
        }

        let step_total = Vec3::new(step_resolved.x, up_vel + down_vel, step_resolved.z);

        let step_h_dist = step_total.x * step_total.x + step_total.z * step_total.z;
        let orig_h_dist = resolved.x * resolved.x + resolved.z * resolved.z;

        if step_h_dist > orig_h_dist {
            let step_on_ground = down_vel != -(up_vel - velocity.y);
            return (step_total, step_on_ground || on_ground);
        }
    }

    (resolved, on_ground)
}
