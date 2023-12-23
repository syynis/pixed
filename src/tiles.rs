use bevy::{
    asset::{AssetLoader, AsyncReadExt},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::texture::{CompressedImageFormats, ImageType},
};
use serde::{Deserialize, Serialize};

pub const TILE_SIZE: usize = 20;
pub const HALF_TILE_SIZE: usize = TILE_SIZE / 2;
pub const PIXEL_SIZE: usize = 4;

#[derive(Default, Debug, Reflect, Copy, Clone, PartialEq)]
pub enum TilePixel {
    #[default]
    Up = 0,
    Neutral = 1,
    Down = 2,
    None = 3,
}

#[derive(Debug, Clone)]
pub struct TileLayer {
    pub colors: Vec<TilePixel>,
}

#[derive(Debug, Clone)]
struct SubTile {
    quadrant: usize,
    data: Vec<TileLayer>,
}

impl SubTile {
    pub fn get(&self, adj: &[bool]) -> &TileLayer {
        let horizontal = adj[2 * (self.quadrant % 2)];
        let vertical = adj[2 * (1 - (self.quadrant % 2))];
        let diagonal = adj[1];

        let idx = match (horizontal, vertical, diagonal) {
            (true, true, true) => 4,    // All neighbors
            (true, true, false) => 3,   // Only cardinal neighbors
            (true, false, _) => 2,      // Horizontal neighbor
            (false, true, _) => 1,      // Vertical neighbor
            (false, false, true) => 0,  // Only diagonal neighbor
            (false, false, false) => 0, // No neighbors
        };

        &self.data[idx]
    }
}

#[derive(Debug, Clone, Asset, TypePath, TypeUuid)]
#[uuid = "70ed170c-1a17-4931-a91a-91737ccdcd9e"]
pub struct TileTexture {
    pub texture: TileLayer,
    pub size: UVec2,
    pub filter: Vec<TilePixel>,
    pub layers: Vec<usize>,
}

impl TileTexture {
    pub fn get_pixel(&self, pos: UVec2) -> TilePixel {
        let pos = UVec2::new(
            pos.x % (self.size.x * TILE_SIZE as u32),
            pos.y % (self.size.y * TILE_SIZE as u32),
        );
        let idx = pos.x + pos.y * TILE_SIZE as u32 * self.size.x;
        self.texture.colors[idx as usize]
    }
}

#[derive(Debug, TypeUuid, TypePath, Asset)]
#[uuid = "ec862102-7e75-433a-88eb-dd684038c6db"]
pub struct Material {
    pub block: BlockMaterial,
}

#[derive(Debug, Clone)]
pub struct BlockMaterial {
    // 0: NW, 1: NE, 2: SE, 3: SW
    sub_tiles: Vec<SubTile>,
    computed_tile_layers: Vec<usize>,
}

impl BlockMaterial {
    pub fn get_pixel(&self, sub_layer: usize, rpos: usize, neighbors: &[bool]) -> TilePixel {
        let Some(_tile_layer) = self.computed_tile_layers.get(sub_layer) else {
            return TilePixel::Neutral;
        };
        let (x, y) = (rpos % TILE_SIZE, rpos / TILE_SIZE);

        let quadrant = match (x, y) {
            (0..=9, 0..=9) => 0,     // Top left
            (10..=19, 0..=9) => 1,   // Top right
            (10..=19, 10..=19) => 2, // Bottom right
            (0..=9, 10..=19) => 3,   // Bottom left
            _ => unreachable!(),
        };

        let x = if x > 9 { x - 10 } else { x };
        let y = if y > 9 { y - 10 } else { y };
        let idx = x + y * HALF_TILE_SIZE;

        self.sub_tiles[quadrant]
            .get(&neighbors[(quadrant * 2)..=(quadrant * 2 + 2)])
            .colors[idx]
    }
}

#[derive(Debug, TypeUuid, TypePath, Asset)]
#[uuid = "da298345-6c7b-42fb-a39d-a7e711e0abb0"]
pub struct Tile {
    pub layers: Vec<TileLayer>,
    computed_tile_layers: Vec<usize>,
    size: UVec2,
}

impl Tile {
    pub fn get_pixel(&self, sub_layer: usize, rpos: usize) -> TilePixel {
        self.computed_tile_layers
            .get(sub_layer)
            .map_or(TilePixel::Neutral, |tile_layer| {
                self.layers[*tile_layer].colors[rpos]
            })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TileMeta {
    pub name: String,
    pub size: UVec2,
    pub layer_repeats: Vec<usize>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TextureMeta {
    pub name: String,
    pub size: UVec2,
    pub layers: Vec<usize>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MaterialMeta {
    pub name: String,
    pub layer_repeats: Vec<usize>,
}

#[derive(Default)]
pub struct TileLoader;

#[derive(Default)]
pub struct MaterialLoader;

#[derive(Default)]
pub struct TextureLoader;

impl AssetLoader for TileLoader {
    type Asset = Tile;
    type Settings = TileMeta;
    type Error = anyhow::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            let tile_image = Image::from_buffer(
                &bytes,
                ImageType::Extension("png"),
                CompressedImageFormats::NONE,
                true,
                bevy::render::texture::ImageSampler::Default,
            )?;

            let tile = Tile {
                layers: TileLayer::from(&tile_image.data)
                    .colors
                    .chunks(TILE_SIZE.pow(2) * (settings.size.x * settings.size.y) as usize)
                    .map(|chunk| TileLayer {
                        colors: chunk.to_vec(),
                    })
                    .collect(),
                computed_tile_layers: compute_tile_layers(&settings.layer_repeats),
                size: settings.size,
            };
            Ok(tile)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["png"]
    }
}

impl AssetLoader for MaterialLoader {
    type Asset = Material;
    type Settings = MaterialMeta;
    type Error = anyhow::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            let tile_image = Image::from_buffer(
                &bytes,
                ImageType::Extension("png"),
                CompressedImageFormats::NONE,
                true,
                bevy::render::texture::ImageSampler::Default,
            )?;

            let computed_tile_layers = compute_tile_layers(&settings.layer_repeats);
            let block_rows = 5; // 5 cases A = AIR, S = SOLID : (A,A), (A,S), (S,A), (A,A,D), (S,S)
            let sub_tiles: Vec<SubTile> = TileLayer::from(&tile_image.data)
                .colors
                .chunks(HALF_TILE_SIZE.pow(2) * block_rows)
                .enumerate()
                .map(|(quadrant, row)| SubTile {
                    quadrant,
                    data: (0..block_rows)
                        .map(|r| {
                            let start = r * HALF_TILE_SIZE;
                            TileLayer {
                                colors: (0..HALF_TILE_SIZE)
                                    .flat_map(move |vy| (0..HALF_TILE_SIZE).map(move |vx| (vx, vy)))
                                    .map(|(vx, vy)| {
                                        let wpos = start + vx + vy * HALF_TILE_SIZE * block_rows;
                                        row[wpos]
                                    })
                                    .collect(),
                            }
                        })
                        .collect::<Vec<TileLayer>>(),
                })
                .collect();
            let block = BlockMaterial {
                sub_tiles,
                computed_tile_layers,
            };

            let material = Material { block };

            Ok(material)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["png"]
    }
}

impl AssetLoader for TextureLoader {
    type Asset = TileTexture;
    type Settings = TextureMeta;
    type Error = anyhow::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            let tile_image = Image::from_buffer(
                &bytes,
                ImageType::Extension("png"),
                CompressedImageFormats::NONE,
                true,
                bevy::render::texture::ImageSampler::Default,
            )?;
            let texture = TileTexture {
                texture: TileLayer::from(&tile_image.data),
                size: settings.size,
                layers: settings.layers.clone(),
                filter: vec![TilePixel::None, TilePixel::Neutral],
            };

            Ok(texture)
        })
    }

    fn extensions(&self) -> &[&str] {
        todo!()
    }
}

fn compute_tile_layers(layer_repeats: &Vec<usize>) -> Vec<usize> {
    let get_tile_layer = |sub_layer: usize, layer_repeats: &Vec<usize>| -> usize {
        let mut unrolled = Vec::new();
        for (idx, e) in layer_repeats.iter().enumerate() {
            (0..*e).for_each(|_| {
                unrolled.push(idx);
            })
        }
        unrolled[sub_layer]
    };
    (0..layer_repeats.iter().sum())
        .map(|idx| get_tile_layer(idx, layer_repeats))
        .collect()
}

impl From<&Vec<u8>> for TileLayer {
    fn from(value: &Vec<u8>) -> Self {
        TileLayer {
            colors: value
                .chunks(PIXEL_SIZE)
                .map(|pixel| {
                    let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);
                    match (r, g, b, a) {
                        (0, 0, 0, 0) => TilePixel::None,
                        (0, 0, 255, 255) => TilePixel::Up,
                        (0, 255, 0, 255) => TilePixel::Neutral,
                        (255, 0, 0, 255) => TilePixel::Down,
                        _ => unreachable!(),
                    }
                })
                .collect::<Vec<TilePixel>>(),
        }
    }
}
