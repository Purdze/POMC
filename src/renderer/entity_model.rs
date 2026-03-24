use glam::Vec3;

use super::chunk::mesher::ChunkVertex;

pub struct ModelCube {
    pub origin: Vec3,
    pub size: Vec3,
    pub tex_offset: (u32, u32),
    pub mirror: bool,
}

pub struct ModelPart {
    pub cubes: Vec<ModelCube>,
    pub children: Vec<(String, ModelPart)>,
    pub offset: Vec3,
    pub rotation: Vec3,
}

impl ModelPart {
    fn new(offset: Vec3) -> Self {
        Self {
            cubes: Vec::new(),
            children: Vec::new(),
            offset,
            rotation: Vec3::ZERO,
        }
    }
}

pub fn build_pig_model() -> ModelPart {
    let mut root = ModelPart::new(Vec3::ZERO);

    let mut head = ModelPart::new(Vec3::new(0.0, 12.0, -6.0));
    head.cubes.push(ModelCube {
        origin: Vec3::new(-4.0, -4.0, -8.0),
        size: Vec3::new(8.0, 8.0, 8.0),
        tex_offset: (0, 0),
        mirror: false,
    });
    head.cubes.push(ModelCube {
        origin: Vec3::new(-2.0, 0.0, -9.0),
        size: Vec3::new(4.0, 3.0, 1.0),
        tex_offset: (16, 16),
        mirror: false,
    });
    root.children.push(("head".into(), head));

    let mut body = ModelPart::new(Vec3::new(0.0, 11.0, 2.0));
    body.rotation = Vec3::new(std::f32::consts::FRAC_PI_2, 0.0, 0.0);
    body.cubes.push(ModelCube {
        origin: Vec3::new(-5.0, -10.0, -7.0),
        size: Vec3::new(10.0, 16.0, 8.0),
        tex_offset: (28, 8),
        mirror: false,
    });
    root.children.push(("body".into(), body));

    let leg_tex = (0, 16);
    let leg_size = Vec3::new(4.0, 6.0, 4.0);
    let leg_origin = Vec3::new(-2.0, 0.0, -2.0);

    for (name, offset) in [
        ("right_hind_leg", Vec3::new(-3.0, 18.0, 7.0)),
        ("left_hind_leg", Vec3::new(3.0, 18.0, 7.0)),
        ("right_front_leg", Vec3::new(-3.0, 18.0, -5.0)),
        ("left_front_leg", Vec3::new(3.0, 18.0, -5.0)),
    ] {
        let mut leg = ModelPart::new(offset);
        leg.cubes.push(ModelCube {
            origin: leg_origin,
            size: leg_size,
            tex_offset: leg_tex,
            mirror: false,
        });
        root.children.push((name.into(), leg));
    }

    root
}

pub fn build_baby_pig_model() -> ModelPart {
    let adult = build_pig_model();
    let mut root = ModelPart::new(Vec3::ZERO);

    for (name, mut part) in adult.children {
        if name == "head" {
            part.offset.y += 4.0;
            part.offset.z += 4.0;
            root.children.push((name, part));
        } else {
            part.offset.y += 24.0;
            part.offset = Vec3::new(
                part.offset.x * 0.5,
                part.offset.y * 0.5,
                part.offset.z * 0.5,
            );
            for cube in &mut part.cubes {
                cube.origin *= 0.5;
                cube.size *= 0.5;
            }
            root.children.push((name, part));
        }
    }

    root
}

pub fn setup_quadruped_anim(
    model: &mut ModelPart,
    head_pitch: f32,
    head_yaw: f32,
    walk_pos: f32,
    walk_speed: f32,
) {
    for (name, part) in &mut model.children {
        match name.as_str() {
            "head" => {
                part.rotation.x = head_pitch.to_radians();
                part.rotation.y = head_yaw.to_radians();
            }
            "right_hind_leg" => {
                part.rotation.x = (walk_pos * 0.6662).cos() * 1.4 * walk_speed;
            }
            "left_hind_leg" => {
                part.rotation.x =
                    (walk_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * walk_speed;
            }
            "right_front_leg" => {
                part.rotation.x =
                    (walk_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * walk_speed;
            }
            "left_front_leg" => {
                part.rotation.x = (walk_pos * 0.6662).cos() * 1.4 * walk_speed;
            }
            _ => {}
        }
    }
}

pub fn generate_entity_vertices(model: &ModelPart, tex_w: u32, tex_h: u32) -> Vec<ChunkVertex> {
    let mut vertices = Vec::new();
    generate_part_vertices(model, tex_w, tex_h, &glam::Mat4::IDENTITY, &mut vertices);
    vertices
}

fn generate_part_vertices(
    part: &ModelPart,
    tex_w: u32,
    tex_h: u32,
    parent_transform: &glam::Mat4,
    vertices: &mut Vec<ChunkVertex>,
) {
    let offset = Vec3::new(part.offset.x, -(part.offset.y - 24.0), part.offset.z) / 16.0;
    let local = glam::Mat4::from_translation(offset)
        * glam::Mat4::from_rotation_x(-part.rotation.x)
        * glam::Mat4::from_rotation_y(part.rotation.y)
        * glam::Mat4::from_rotation_z(part.rotation.z);
    let transform = *parent_transform * local;

    for cube in &part.cubes {
        generate_cube_vertices(cube, tex_w, tex_h, &transform, vertices);
    }

    for (_name, child) in &part.children {
        generate_part_vertices(child, tex_w, tex_h, &transform, vertices);
    }
}

fn generate_cube_vertices(
    cube: &ModelCube,
    tex_w: u32,
    tex_h: u32,
    transform: &glam::Mat4,
    vertices: &mut Vec<ChunkVertex>,
) {
    let tw = tex_w as f32;
    let th = tex_h as f32;
    let u0 = cube.tex_offset.0 as f32;
    let v0 = cube.tex_offset.1 as f32;
    let w = cube.size.x;
    let h = cube.size.y;
    let d = cube.size.z;

    let x0 = cube.origin.x / 16.0;
    let y0 = cube.origin.y / 16.0;
    let z0 = cube.origin.z / 16.0;
    let x1 = x0 + w / 16.0;
    let y1 = y0 + h / 16.0;
    let z1 = z0 + d / 16.0;

    let y0_flipped = -y1;
    let y1_flipped = -y0;

    struct Face {
        positions: [[f32; 3]; 4],
        uv: [f32; 4],
    }

    let (right_uv, left_uv) = if cube.mirror {
        (
            [u0 + d + w, v0 + d, u0 + d + w + d, v0 + d + h],
            [u0, v0 + d, u0 + d, v0 + d + h],
        )
    } else {
        (
            [u0, v0 + d, u0 + d, v0 + d + h],
            [u0 + d + w, v0 + d, u0 + d + w + d, v0 + d + h],
        )
    };

    let faces = [
        Face {
            positions: [
                [x1, y0_flipped, z0],
                [x0, y0_flipped, z0],
                [x0, y1_flipped, z0],
                [x1, y1_flipped, z0],
            ],
            uv: [u0 + d, v0 + d, u0 + d + w, v0 + d + h],
        },
        Face {
            positions: [
                [x0, y0_flipped, z1],
                [x1, y0_flipped, z1],
                [x1, y1_flipped, z1],
                [x0, y1_flipped, z1],
            ],
            uv: [u0 + d + w + d, v0 + d, u0 + d + w + d + w, v0 + d + h],
        },
        Face {
            positions: [
                [x0, y1_flipped, z0],
                [x0, y1_flipped, z1],
                [x1, y1_flipped, z1],
                [x1, y1_flipped, z0],
            ],
            uv: [u0 + d, v0, u0 + d + w, v0 + d],
        },
        Face {
            positions: [
                [x0, y0_flipped, z1],
                [x0, y0_flipped, z0],
                [x1, y0_flipped, z0],
                [x1, y0_flipped, z1],
            ],
            uv: [u0 + d + w, v0, u0 + d + w + w, v0 + d],
        },
        Face {
            positions: [
                [x0, y0_flipped, z1],
                [x0, y0_flipped, z0],
                [x0, y1_flipped, z0],
                [x0, y1_flipped, z1],
            ],
            uv: right_uv,
        },
        Face {
            positions: [
                [x1, y0_flipped, z0],
                [x1, y0_flipped, z1],
                [x1, y1_flipped, z1],
                [x1, y1_flipped, z0],
            ],
            uv: left_uv,
        },
    ];

    for face in &faces {
        let u_min = face.uv[0] / tw;
        let v_min = face.uv[1] / th;
        let u_max = face.uv[2] / tw;
        let v_max = face.uv[3] / th;

        let uvs = [
            [u_min, v_max],
            [u_max, v_max],
            [u_max, v_min],
            [u_min, v_min],
        ];

        for &i in &[0usize, 1, 2, 0, 2, 3] {
            let p = transform.transform_point3(Vec3::from(face.positions[i]));
            vertices.push(ChunkVertex {
                position: p.into(),
                tex_coords: uvs[i],
                light: 1.0,
                tint: [1.0, 1.0, 1.0],
            });
        }
    }
}
