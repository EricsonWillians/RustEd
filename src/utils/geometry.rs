// src/utils/geometry.rs
#[derive(Debug, Clone)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point2D) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone)]
pub struct Line2D {
    pub start: Point2D,
    pub end: Point2D,
}

impl Line2D {
    pub fn new(start: Point2D, end: Point2D) -> Self {
        Self { start, end }
    }

    pub fn length(&self) -> f64 {
        self.start.distance_to(&self.end)
    }

    pub fn distance_to_point(&self, point: &Point2D) -> f64 {
        let line_length = self.length();
        if line_length == 0.0 {
            return point.distance_to(&self.start);
        }

        // Calculate distance using the cross product
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let abs_distance = ((dy * point.x - dx * point.y + 
                           self.end.x * self.start.y - 
                           self.end.y * self.start.x).abs()) / line_length;

        abs_distance
    }
}

#[derive(Debug, Clone)]
pub struct Vector2D {
    pub x: f64,
    pub y: f64,
}

impl Vector2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn dot(&self, other: &Vector2D) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn normalize(&self) -> Vector2D {
        let length = (self.x * self.x + self.y * self.y).sqrt();
        if length == 0.0 {
            return self.clone();
        }
        Vector2D::new(self.x / length, self.y / length)
    }
}