//! Layer band rectangles share the node motion clock and stable architectural identity.
use super::Spring;
use crate::projection::LayerBand;
use std::collections::HashMap;

type Key = (String, Option<String>, bool, Option<String>);
fn key(band: &LayerBand) -> Key {
    (
        band.name.clone(),
        band.flow.clone(),
        band.support,
        band.beside.clone(),
    )
}

#[derive(Default)]
pub(super) struct Bands {
    tracks: Vec<[Spring; 4]>,
}
impl Bands {
    pub fn begin(&mut self, before: Vec<LayerBand>, target: &[LayerBand]) {
        let old: HashMap<_, _> = before
            .into_iter()
            .map(|b| {
                (
                    key(&b),
                    [b.position[0], b.position[1], b.size[0], b.size[1]],
                )
            })
            .collect();
        self.tracks = target
            .iter()
            .map(|band| {
                let rect = [
                    band.position[0],
                    band.position[1],
                    band.size[0],
                    band.size[1],
                ];
                let origin = old.get(&key(band)).copied().unwrap_or(rect);
                std::array::from_fn(|i| Spring::new(origin[i], rect[i]))
            })
            .collect();
    }
    pub fn advance(&mut self, dt: f32) -> bool {
        let mut active = false;
        for track in self.tracks.iter_mut().flatten() {
            active |= track.step(dt);
        }
        active
    }
    pub fn apply(&self, bands: &mut [LayerBand]) {
        for (band, tracks) in bands.iter_mut().zip(&self.tracks) {
            band.position = [tracks[0].value, tracks[1].value];
            band.size = [tracks[2].value, tracks[3].value];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layer_bands_follow_shared_clock_by_architectural_identity() {
        let mut bands = Bands::default();
        let old = LayerBand {
            name: "service".into(),
            position: [0.0, 0.0],
            size: [100.0, 100.0],
            ..Default::default()
        };
        let mut target = vec![LayerBand {
            name: "service".into(),
            position: [200.0, 100.0],
            size: [300.0, 120.0],
            ..Default::default()
        }];
        bands.begin(vec![old], &target);
        bands.apply(&mut target);
        assert_eq!(target[0].position, [0.0, 0.0]);
        bands.advance(0.03);
        bands.apply(&mut target);
        assert!(target[0].position[0] > 0.0 && target[0].position[0] < 200.0);
    }
}
