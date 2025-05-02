use crate::{
    chunk_db::{ChunkLoaderSaver, SimpleToFileSaver},
    Chunk, ChunkCoordinates,
};

use paintplayground::types::*;

pub struct Screenshot {
    chunks: Vec<Vec<Option<Chunk>>>,
}

impl Screenshot {
    /// create screenshot from two corner [`ChunkCoordinates`]
    ///
    /// todo use [`AppState`] to get the chunks, instead of filesaver
    pub fn from_coordinates(top_left: ChunkCoordinates, bottom_right: ChunkCoordinates) -> Self {
        let loader = SimpleToFileSaver::new();

        let min_x = top_left.x().min(bottom_right.x());
        let max_x = top_left.x().max(bottom_right.x());
        let min_y = bottom_right.y().min(top_left.y());
        let max_y = bottom_right.y().max(top_left.y());

        let width = (max_x - min_x + 1) as usize;
        let height = (max_y - min_y + 1) as usize;

        let mut chunks = Vec::with_capacity(height);

        for y in (min_y..=max_y).rev() {
            let mut row = Vec::with_capacity(width);
            for x in min_x..=max_x {
                let coordinate = ChunkCoordinates::new(x, y).unwrap();
                if let Ok(chunk) = loader.load_chunk(coordinate, false) {
                    row.push(Some(chunk));
                } else {
                    row.push(None);
                }
            }
            chunks.push(row);
        }

        Self { chunks }
    }

    /// create [`Screenshot`] from chunks
    pub fn from_chunks(chunks: Vec<Vec<Option<Chunk>>>) -> Self {
        Self { chunks }
    }

    /// generate a rbg8 buffer from the chunks,
    /// returns buffer, width, height
    pub fn generate_buffer(&self, quality: u8) -> (Vec<u8>, u32, u32) {
        let x_chunks = self.chunks[0].len();
        let y_chunks = self.chunks.len();

        let scale = quality.max(1) as usize;
        let chunk_scaled = CHUNK_LENGTH * scale;
        let img_width = (x_chunks * chunk_scaled) as u32;
        let img_height = (y_chunks * chunk_scaled) as u32;

        let buffer_size = (img_width * img_height * 3) as usize;
        let mut buffer = vec![0u8; buffer_size];

        self.chunks
            .iter()
            .enumerate()
            .for_each(|(chunk_y, chunk_row)| {
                for row_in_chunk in 0..CHUNK_LENGTH {
                    let base_y = (chunk_y * CHUNK_LENGTH + row_in_chunk) * scale;

                    chunk_row
                        .iter()
                        .enumerate()
                        .for_each(|(chunk_x, maybe_chunk)| {
                            let row_colors = match maybe_chunk {
                                Some(chunk) => chunk.row_of_colors(row_in_chunk),
                                None => vec![Color::Zero; CHUNK_LENGTH],
                            };

                            let base_x = chunk_x * chunk_scaled;

                            for (x, color) in row_colors.iter().enumerate() {
                                let (r, g, b) = color.to_rgb();

                                // scaling, how fun....
                                // ? perhaps we don't want this?
                                for dy in 0..scale {
                                    let y_offset = base_y + dy;
                                    for dx in 0..scale {
                                        let x_offset = base_x + x * scale + dx;
                                        let pixel_index =
                                            ((y_offset * img_width as usize + x_offset) * 3);
                                        buffer[pixel_index] = r;
                                        buffer[pixel_index + 1] = g;
                                        buffer[pixel_index + 2] = b;
                                    }
                                }
                            }
                        });
                }
            });

        (buffer, img_width, img_height)
    }

    pub fn create_png(self, quality: u8) -> Vec<u8> {
        let (buffer, width, height) = self.generate_buffer(quality);
        let mut png_buffer = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_buffer, width, height);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&buffer).unwrap();
        }

        png_buffer
    }

    /// save chunks screenshot to file
    pub fn save(&self, quality: u8, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (buffer, img_width, img_height) = self.generate_buffer(quality);

        image::save_buffer(
            filename,
            &buffer,
            img_width,
            img_height,
            image::ColorType::Rgb8,
        )?;

        Ok(())
    }
}
