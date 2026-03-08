use azalea_core::position::ChunkPos;
use azalea_protocol::packets::game::{ClientboundGamePacket, ServerboundGamePacket};
use crossbeam_channel::Sender;

use super::sender::PacketSender;
use super::NetworkEvent;

pub fn handle_game_packet(
    packet: &ClientboundGamePacket,
    sender: &PacketSender,
    event_tx: &Sender<NetworkEvent>,
) {
    match packet {
        ClientboundGamePacket::LevelChunkWithLight(p) => {
            log::debug!(
                "Chunk [{}, {}] ({} block entities)",
                p.x,
                p.z,
                p.chunk_data.block_entities.len()
            );
            let _ = event_tx.try_send(NetworkEvent::ChunkLoaded {
                pos: ChunkPos::new(p.x, p.z),
                data: p.chunk_data.data.clone(),
                heightmaps: p.chunk_data.heightmaps.clone(),
            });
        }
        ClientboundGamePacket::ForgetLevelChunk(p) => {
            let _ = event_tx.try_send(NetworkEvent::ChunkUnloaded { pos: p.pos });
        }
        ClientboundGamePacket::SetChunkCacheCenter(p) => {
            let _ = event_tx.try_send(NetworkEvent::ChunkCacheCenter { x: p.x, z: p.z });
        }
        ClientboundGamePacket::PlayerPosition(p) => {
            let _ = event_tx.try_send(NetworkEvent::PlayerPosition {
                x: p.change.pos.x,
                y: p.change.pos.y,
                z: p.change.pos.z,
                yaw: p.change.look_direction.y_rot(),
                pitch: p.change.look_direction.x_rot(),
            });
            sender.send(ServerboundGamePacket::AcceptTeleportation(
                azalea_protocol::packets::game::s_accept_teleportation::ServerboundAcceptTeleportation {
                    id: p.id,
                },
            ));
        }
        ClientboundGamePacket::KeepAlive(p) => {
            sender.send(ServerboundGamePacket::KeepAlive(
                azalea_protocol::packets::game::s_keep_alive::ServerboundKeepAlive { id: p.id },
            ));
        }
        ClientboundGamePacket::ChunkBatchFinished(p) => {
            sender.send(ServerboundGamePacket::ChunkBatchReceived(
                azalea_protocol::packets::game::s_chunk_batch_received::ServerboundChunkBatchReceived {
                    desired_chunks_per_tick: p.batch_size as f32,
                },
            ));
        }
        ClientboundGamePacket::Disconnect(p) => {
            log::warn!("Disconnected: {}", p.reason);
            let _ = event_tx.try_send(NetworkEvent::Disconnected {
                reason: format!("{}", p.reason),
            });
        }
        other => {
            log::debug!("Game packet: {:?}", std::mem::discriminant(other));
        }
    }
}
