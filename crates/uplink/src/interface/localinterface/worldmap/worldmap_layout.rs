//! Port of source/interface/localinterface/worldmap/worldmap_layout.h + source/interface/localinterface/worldmap/worldmap_layout.cpp
//! Places the world map's location labels with a simulated-annealing layout.

use std::time::Instant;

use uplink_engine::{gl, text};

use crate::interface::localinterface::worldmap::rectangle::MapRectangle;
use crate::world::generator::numbergenerator::NumberGenerator;

/// C++ `#define WORLDMAPOBJECT_*`: kinds of world map object.
pub const WORLDMAPOBJECT_NONE: i32 = 0;
pub const WORLDMAPOBJECT_LABEL: i32 = 1;
pub const WORLDMAPOBJECT_LOCATION: i32 = 2;
pub const WORLDMAPOBJECT_GATEWAY: i32 = 3;

/// C++ `class WorldMapInterfaceObject`: a point on the map (location or gateway marker).
#[derive(Debug, Clone, Default)]
pub struct WorldMapInterfaceObject {
    /// C++ `int TYPE` (WORLDMAPOBJECT_*).
    pub typ: i32,
    pub x: i32,
    pub y: i32,
    /// C++ `baseX`, `baseY`: the point this object is attached to.
    pub base_x: i32,
    pub base_y: i32,
}

impl WorldMapInterfaceObject {
    /// C++ `WorldMapInterfaceObject::WorldMapInterfaceObject()`
    pub fn new() -> Self {
        Self::default()
    }

    /// C++ `SetTYPE`
    pub fn set_type(&mut self, new_type: i32) {
        self.typ = new_type;
    }

    /// C++ `SetPosition`
    pub fn set_position(&mut self, newx: i32, newy: i32) {
        self.x = newx;
        self.y = newy;
    }

    /// C++ `SetBasePosition`
    pub fn set_base_position(&mut self, newx: i32, newy: i32) {
        self.base_x = newx;
        self.base_y = newy;
    }

    /// C++ virtual `Draw(int xOffset = 0, int yOffset = 0, float zoom = 1.0)`
    pub fn draw(&self, x_offset: i32, y_offset: i32, zoom: f32) {
        match self.typ {
            WORLDMAPOBJECT_GATEWAY | WORLDMAPOBJECT_LOCATION => {
                // C++ falls through from GATEWAY into LOCATION (see the DON'T INSERT note there).
                gl::color4f(1.0, 1.0, 1.0, 1.0);
                let x_pos = (23.0_f32 + ((self.x - 23) - x_offset) as f32 * zoom) as i32;
                let y_pos = (50.0_f32 + ((self.y - 50) - y_offset) as f32 * zoom) as i32;

                if x_pos >= 0 && y_pos >= 0 {
                    gl::begin(gl::QUADS);
                    gl::vertex2i(x_pos, y_pos);
                    gl::vertex2i(x_pos + 7, y_pos);
                    gl::vertex2i(x_pos + 7, y_pos + 7);
                    gl::vertex2i(x_pos, y_pos + 7);
                    gl::end();
                }
            }
            _ => {}
        }
    }

    /// C++ virtual `GetExtent() const`
    pub fn get_extent(&self) -> MapRectangle {
        MapRectangle::new(self.x, self.y, 7, 7)
    }
}

/// C++ `class WorldMapInterfaceLabel : WorldMapInterfaceObject`: a caption placed beside a feature point.
// PORT: the C++ subclass is composed: `base` holds the WorldMapInterfaceObject part.
#[derive(Debug, Clone)]
pub struct WorldMapInterfaceLabel {
    pub base: WorldMapInterfaceObject,
    /// C++ `char *caption`
    pub caption: String,
    /// C++ `WorldMapInterfaceObject *featurePoint`, non-owning. Stored as the feature point's extent:
    /// the feature point is only placed once (in `WorldMapLayout::add_location`) and never moves,
    /// so its `GetExtent()` is fixed for the label's whole life.
    // PORT: non-owning pointer replaced by a copy of the feature point's extent.
    pub feature_extent: MapRectangle,
    /// C++ `int labelPos`: which candidate position is in use.
    pub label_pos: i32,
    /// C++ `int labelWidth`
    pub label_width: i32,
    /// C++ `int numPossLabelPos`
    pub num_poss_label_pos: i32,
    /// C++ `int possLabelPos[8]`
    pub poss_label_pos: [i32; 8],
}

impl WorldMapInterfaceLabel {
    /// C++ `WorldMapInterfaceLabel(const MapRectangle &mapRect, WorldMapInterfaceObject *newFeaturePoint, const char *newCaption)`
    pub fn new(map_rect: &MapRectangle, feature_point: &WorldMapInterfaceObject, caption: &str) -> Self {
        let mut label = WorldMapInterfaceLabel {
            base: WorldMapInterfaceObject::default(),
            caption: String::new(),
            feature_extent: feature_point.get_extent(),
            label_pos: 0,
            label_width: 0,
            num_poss_label_pos: 0,
            poss_label_pos: [0; 8],
        };
        label.base.set_type(WORLDMAPOBJECT_LABEL);
        label.set_caption(caption);
        label.calculate_possible_label_positions(map_rect);
        label.set_random_label_position();
        label
    }

    /// C++ `SetCaption`
    pub fn set_caption(&mut self, newcaption: &str) {
        self.caption = newcaption.to_string();
        self.calculate_width();
    }

    /// C++ `SetLabelPosition(int n)`
    pub fn set_label_position(&mut self, n: i32) {
        let fp = self.feature_extent;
        let cp = self.get_extent();

        self.label_pos = n;

        match n {
            0 => {
                // Top Centre
                self.base.set_position(fp.x1 - cp.width / 2, fp.y1 - cp.height - 1);
            }
            1 => {
                // Bottom Centre
                self.base.set_position(fp.x1 - cp.width / 2, fp.y2() + 1);
            }
            2 => {
                // Bottom Right
                self.base.set_position(fp.x2() + 1, fp.y2() + 1);
            }
            3 => {
                // Right
                self.base.set_position(fp.x2() + 2, fp.y1);
            }
            4 => {
                // Top Right
                self.base.set_position(fp.x2() + 1, fp.y1 - cp.height + 1);
            }
            5 => {
                // Top Left
                self.base.set_position(fp.x1 - cp.width, fp.y1 - cp.height + 1);
            }
            6 => {
                // Left
                self.base.set_position(fp.x1 - cp.width - 2, fp.y1);
            }
            7 => {
                // Bottom Left
                self.base.set_position(fp.x1 - cp.width, fp.y2() + 1);
            }
            _ => debug_assert!(false, "WorldMapInterfaceLabel::SetLabelPosition: bad position {n}"),
        }
    }

    /// C++ `SetRandomLabelPosition()`
    pub fn set_random_label_position(&mut self) {
        // When no candidate position fits, the C++ reads possLabelPos[RandomNumber(0)], i.e. index
        // -1: in the class layout that is the int just before the array, numPossLabelPos (0).
        let idx = NumberGenerator::random_number(self.num_poss_label_pos);
        let pos = if idx < 0 { self.num_poss_label_pos } else { self.poss_label_pos[idx as usize] };
        self.set_label_position(pos);
    }

    /// C++ `GetLabelPosition() const`
    pub fn get_label_position(&self) -> i32 {
        self.label_pos
    }

    /// C++ `GetExtent() const` (overrides the base version)
    pub fn get_extent(&self) -> MapRectangle {
        MapRectangle::new(self.base.x, self.base.y, self.label_width, 13)
    }

    /// C++ `Overlaps(WorldMapInterfaceObject *label) const`
    pub fn overlaps(&self, label: &WorldMapInterfaceLabel) -> bool {
        self.get_extent().intersects(&label.get_extent())
    }

    /// C++ `Draw(int xOffset = 0, int yOffset = 0, float zoom = 1.0)` (overrides the base version)
    pub fn draw(&self, x_offset: i32, y_offset: i32, zoom: f32) {
        if self.base.typ != WORLDMAPOBJECT_LABEL {
            // C++ `default: UplinkAssert(0)`
            debug_assert!(false, "WorldMapInterfaceLabel::Draw: unexpected TYPE {}", self.base.typ);
            return;
        }

        let dx = self.base.x - self.base.base_x;
        let dy = self.base.y - self.base.base_y;

        let mut x_pos = (23.0_f32 + ((self.base.base_x - 23) - x_offset) as f32 * zoom) as i32;
        let mut y_pos = (50.0_f32 + ((self.base.base_y - 50) - y_offset) as f32 * zoom) as i32;

        x_pos += dx;
        y_pos += dy;

        if x_pos >= 0 && y_pos >= 0 {
            gl::color4f(1.0, 1.0, 1.0, 1.0);
            text::draw_text_default(x_pos, y_pos + 7, &self.caption);
        }
    }

    /// C++ `GetCaption() const`
    pub fn get_caption(&self) -> &str {
        &self.caption
    }

    /// C++ protected `CalculateWidth()`
    fn calculate_width(&mut self) {
        self.label_width = text::text_width_default(&self.caption);
    }

    /// C++ protected `CalculatePossibleLabelPositions(const MapRectangle &mapRect)`
    fn calculate_possible_label_positions(&mut self, map_rect: &MapRectangle) {
        self.num_poss_label_pos = 0;
        for pos in 0..8 {
            self.set_label_position(pos);
            if map_rect.contains(&self.get_extent()) {
                let slot = self.num_poss_label_pos as usize;
                self.poss_label_pos[slot] = pos;
                self.num_poss_label_pos += 1;
            }
        }
    }
}

/// C++ `class WorldMapObjectiveFunction`: the cost the annealing minimises (label overlap and clipping).
// PORT: `int *shadeScreen` becomes an owned Vec of mapWidth * mapHeight cells.
#[derive(Debug, Clone)]
pub struct WorldMapObjectiveFunction {
    /// C++ `int cost`
    pub cost: i32,
    /// C++ `int mapWidth, mapHeight`
    pub map_width: i32,
    pub map_height: i32,
    /// C++ `int *shadeScreen`
    pub shade_screen: Vec<i32>,
    /// C++ `MapRectangle clipRect`
    pub clip_rect: MapRectangle,
}

/// C++ file-static `penalty(int numOverlaps)`
fn penalty(num_overlaps: i32) -> i32 {
    num_overlaps
}

impl WorldMapObjectiveFunction {
    /// C++ `WorldMapObjectiveFunction(const MapRectangle &mapRect)`
    pub fn new(map_rect: &MapRectangle) -> Self {
        let cells = (map_rect.width * map_rect.height).max(0) as usize;
        let mut f = WorldMapObjectiveFunction {
            cost: 0,
            map_width: map_rect.width,
            map_height: map_rect.height,
            shade_screen: vec![0; cells],
            clip_rect: *map_rect,
        };
        f.reset();
        f
    }

    /// C++ `GetCost() const`
    pub fn get_cost(&self) -> i32 {
        self.cost
    }

    /// C++ `AddObject(const WorldMapInterfaceObject *m)`
    pub fn add_object(&mut self, m: &WorldMapInterfaceObject) {
        self.add_rect(&m.get_extent());
    }

    /// C++ `SubObject(const WorldMapInterfaceObject *m)`
    pub fn sub_object(&mut self, m: &WorldMapInterfaceObject) {
        self.sub_rect(&m.get_extent());
    }

    /// C++ `AddRect(const MapRectangle &r)`
    pub fn add_rect(&mut self, r: &MapRectangle) {
        let mut r = self.clip_rect.intersection(r);

        if r.is_null() {
            return;
        }

        r.x1 -= self.clip_rect.x1;
        r.y1 -= self.clip_rect.y1;

        let x2 = r.x2();
        let y2 = r.y2();

        for y in r.y1..=y2 {
            let line = (y * self.map_width) as usize;
            for x in r.x1..=x2 {
                let cell = line + x as usize;
                self.shade_screen[cell] += 1;
                if self.shade_screen[cell] != 1 {
                    self.cost += penalty(self.shade_screen[cell]);
                }
            }
        }
    }

    /// C++ `SubRect(const MapRectangle &r)`
    pub fn sub_rect(&mut self, r: &MapRectangle) {
        let mut r = self.clip_rect.intersection(r);

        if r.is_null() {
            return;
        }

        r.x1 -= self.clip_rect.x1;
        r.y1 -= self.clip_rect.y1;

        let x2 = r.x2();
        let y2 = r.y2();

        for y in r.y1..=y2 {
            let line = (y * self.map_width) as usize;
            for x in r.x1..=x2 {
                let cell = line + x as usize;
                self.shade_screen[cell] -= 1;
                if self.shade_screen[cell] != 0 {
                    self.cost -= penalty(1 + self.shade_screen[cell]);
                }
            }
        }
    }

    /// C++ `Reset()`
    pub fn reset(&mut self) {
        self.shade_screen.iter_mut().for_each(|cell| *cell = 0);
        self.cost = 0;
    }
}

/// C++ `class WorldMapLayout`: owns the locations and labels and runs the annealing layout.
#[derive(Debug, Clone)]
pub struct WorldMapLayout {
    /// C++ `int iteration`
    pub iteration: i32,
    /// C++ `int moveNumber`
    pub move_number: i32,
    /// C++ `int numGoodMoves`
    pub num_good_moves: i32,
    /// C++ `float T`: temperature.
    pub t: f32,
    /// C++ `float E`: current value of the objective function.
    pub e: f32,
    /// C++ `bool layoutComplete`
    pub layout_complete: bool,
    /// C++ `bool layoutStarted`
    pub layout_started: bool,
    /// C++ `MapRectangle mapRectangle`
    pub map_rectangle: MapRectangle,
    /// C++ `LList<WorldMapInterfaceObject *> locations` (owned).
    pub locations: Vec<WorldMapInterfaceObject>,
    /// C++ `LList<WorldMapInterfaceLabel *> labels` (owned).
    pub labels: Vec<WorldMapInterfaceLabel>,
    /// C++ `WorldMapObjectiveFunction objective`
    pub objective: WorldMapObjectiveFunction,
}

impl WorldMapLayout {
    /// C++ `WorldMapLayout(const MapRectangle &newMapRectangle)`
    pub fn new(new_map_rectangle: &MapRectangle) -> Self {
        let mut layout = WorldMapLayout {
            iteration: 0,
            move_number: 0,
            num_good_moves: 0,
            t: 0.0,
            e: 0.0,
            layout_complete: false,
            layout_started: false,
            map_rectangle: *new_map_rectangle,
            locations: Vec::new(),
            labels: Vec::new(),
            objective: WorldMapObjectiveFunction::new(new_map_rectangle),
        };
        layout.reset();
        layout
    }

    /// C++ `Reset()`
    pub fn reset(&mut self) {
        self.reset_layout_parameters();
        self.delete_locations();
        self.layout_complete = true;
    }

    /// C++ `AddLocation(int x, int y, const char *name)`
    pub fn add_location(&mut self, x: i32, y: i32, name: &str) {
        let locationtype = if name == "Gateway" {
            WORLDMAPOBJECT_GATEWAY
        } else {
            WORLDMAPOBJECT_LOCATION
        };

        let mut thedot = WorldMapInterfaceObject::new();
        thedot.set_type(locationtype);
        thedot.set_position(x, y);
        thedot.set_base_position(x, y);
        self.locations.push(thedot);

        let dot = self.locations.last().expect("just pushed");
        let mut thelabel = WorldMapInterfaceLabel::new(&self.map_rectangle, dot, name);
        thelabel.base.set_base_position(x, y);
        self.labels.push(thelabel);

        if self.layout_started {
            self.reset_layout_parameters();
        }

        self.layout_complete = false;
    }

    /// C++ `PartialLayoutLabels()`
    pub fn partial_layout_labels(&mut self) {
        let n = self.labels.len() as i32;

        if self.layout_complete {
            return;
        }

        if !self.layout_started {
            self.start_layout();
        }

        let mut i = 0;
        while self.e > 0.0 && i < 20 {
            let li = NumberGenerator::random_number(n) as usize;

            let orig_label_pos = self.labels[li].get_label_position();

            self.objective.sub_rect(&self.labels[li].get_extent());
            self.labels[li].set_random_label_position();
            self.objective.add_rect(&self.labels[li].get_extent());

            let mut delta_e: f32 = self.objective.get_cost() as f32 - self.e;

            if delta_e < 0.0 {
                self.num_good_moves += 1;
            } else {
                // C++ `exp(-deltaE/T)`: float argument; <math.h> resolves to the float overload.
                let boltzmann = (-delta_e / self.t).exp();
                if (NumberGenerator::random_uniform_number() as f64) <= 1.0 - boltzmann as f64 {
                    self.objective.sub_rect(&self.labels[li].get_extent());
                    self.labels[li].set_label_position(orig_label_pos);
                    self.objective.add_rect(&self.labels[li].get_extent());

                    delta_e = 0.0;
                }
            }

            self.e += delta_e;
            i += 1;
        }

        self.move_number += 20;

        if self.num_good_moves >= 5 * n || self.move_number >= 10 * n {
            if self.e <= 0.0 || self.num_good_moves <= 0 || self.iteration > 20 {
                self.layout_complete = true;
            }

            self.iteration += 1;
            self.move_number = 0;
            self.num_good_moves = 0;
            // C++ `T -= T / 10.0;` (double arithmetic, stored as float)
            self.t = (self.t as f64 - self.t as f64 / 10.0) as f32;
        }
    }

    /// C++ `FullLayoutLabels()`
    pub fn full_layout_labels(&mut self) {
        // PORT: C++ timed with EclGetAccurateTime(), which lives on the Eclipse object in Ctx and is
        // not reachable from this method. Elapsed wall-clock milliseconds since this call started.
        let start_time = Instant::now();

        while !self.layout_complete && start_time.elapsed().as_millis() < 1500 {
            self.partial_layout_labels();
        }
    }

    /// C++ `IsLayoutComplete() const`
    pub fn is_layout_complete(&self) -> bool {
        self.layout_complete
    }

    /// C++ `GetLocations()`
    pub fn get_locations(&self) -> &[WorldMapInterfaceObject] {
        &self.locations
    }

    /// C++ `GetLabels()`
    pub fn get_labels(&self) -> &[WorldMapInterfaceLabel] {
        &self.labels
    }

    /// C++ protected `StartLayout()`
    fn start_layout(&mut self) {
        self.layout_started = true;

        // Initial Random Layout

        for i in 0..self.labels.len() {
            self.labels[i].set_random_label_position();
            let extent = self.labels[i].get_extent();
            self.objective.add_rect(&extent);
        }

        for il in 0..self.locations.len() {
            let extent = self.locations[il].get_extent();
            self.objective.add_rect(&extent);

            // Add in feature-point neighbourhood

            let mut n = extent;
            n.x1 -= n.width;
            n.y1 -= n.height;
            n.width *= 3;
            n.height *= 3;

            self.objective.add_rect(&n);
        }

        self.e = self.objective.get_cost() as f32;
        self.t = (10.0 / 3.0_f64.ln()) as f32;
    }

    /// C++ protected `DeleteLocations()`
    fn delete_locations(&mut self) {
        self.labels.clear();
        self.locations.clear();
    }

    /// C++ protected `ResetLayoutParameters()`
    fn reset_layout_parameters(&mut self) {
        self.layout_started = false;
        self.iteration = 0;
        self.move_number = 0;
        self.num_good_moves = 0;
        self.t = 0.0;
        self.e = -1.0;
        self.objective.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_label_position_without_candidates_uses_position_zero() {
        // C++ possLabelPos[-1] aliases numPossLabelPos (0) when nothing fits.
        let mut label = WorldMapInterfaceLabel {
            base: WorldMapInterfaceObject::default(),
            caption: String::new(),
            feature_extent: MapRectangle::new(100, 100, 7, 7),
            label_pos: 7,
            label_width: 40,
            num_poss_label_pos: 0,
            poss_label_pos: [0; 8],
        };
        label.set_random_label_position();
        assert_eq!(label.get_label_position(), 0);
    }
}
