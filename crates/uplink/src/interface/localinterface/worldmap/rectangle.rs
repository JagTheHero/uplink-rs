//! Port of source/interface/localinterface/worldmap/rectangle.h
//! Integer rectangle used by the world map layout. Pure logic, so the bodies are real.

/// C++ `class MapRectangle`: origin plus size, in map pixels. `x2`/`y2` are inclusive.
#[derive(Debug, Clone, Copy)]
pub struct MapRectangle {
    pub x1: i32,
    pub y1: i32,
    pub width: i32,
    pub height: i32,
}

impl MapRectangle {
    /// C++ `MapRectangle(int x, int y, int width, int height)`
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        MapRectangle {
            x1: x,
            y1: y,
            width,
            height,
        }
    }

    /// C++ `x2()`: inclusive right edge.
    pub fn x2(&self) -> i32 {
        self.x1 + self.width - 1
    }

    /// C++ `y2()`: inclusive bottom edge.
    pub fn y2(&self) -> i32 {
        self.y1 + self.height - 1
    }

    /// C++ `Min(int a, int b)`
    pub fn min(a: i32, b: i32) -> i32 {
        if a < b {
            a
        } else {
            b
        }
    }

    /// C++ `Max(int a, int b)`
    pub fn max(a: i32, b: i32) -> i32 {
        if a > b {
            a
        } else {
            b
        }
    }

    /// C++ `intersection(const MapRectangle &o)`. May return a rectangle with negative size.
    pub fn intersection(&self, o: &MapRectangle) -> MapRectangle {
        let new_x1 = Self::max(self.x1, o.x1);
        let new_y1 = Self::max(self.y1, o.y1);
        let new_x2 = Self::min(o.x2(), self.x2());
        let new_y2 = Self::min(o.y2(), self.y2());
        MapRectangle::new(new_x1, new_y1, new_x2 - new_x1 + 1, new_y2 - new_y1 + 1)
    }

    /// C++ `isNull()`: true when either size is negative (the intersection is empty).
    pub fn is_null(&self) -> bool {
        self.width < 0 || self.height < 0
    }

    /// C++ `intersects(const MapRectangle &o)`
    pub fn intersects(&self, o: &MapRectangle) -> bool {
        !self.intersection(o).is_null()
    }

    /// C++ `contains(const MapRectangle &o)`: true if `o` lies fully inside this rectangle.
    pub fn contains(&self, o: &MapRectangle) -> bool {
        self.intersection(o) == *o
    }
}

impl PartialEq for MapRectangle {
    /// C++ `operator ==`: compares origin and size.
    fn eq(&self, o: &MapRectangle) -> bool {
        self.x1 == o.x1 && self.y1 == o.y1 && self.width == o.width && self.height == o.height
    }
}
