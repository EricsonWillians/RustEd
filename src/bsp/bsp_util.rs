// src/bsp/bsp_util.rs
// Geometry and other helper functions specific to BSP.

use crate::bsp::{Seg, EPSILON}; // Import from the bsp module
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)] // Add PartialEq
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Point2D { x, y }
    }
}


#[derive(Debug, Clone, Copy)] // Add Copy and Clone since it's just two f64 values.
pub struct Line2D {
    pub start: Point2D,
    pub end: Point2D,
}


impl Line2D {
    pub fn new(start: Point2D, end: Point2D) -> Self {
        Line2D { start, end }
    }

    pub fn from_seg(seg: &Seg) -> Self{
        Line2D{start: seg.start, end: seg.end}
    }

    // Classify a point against the line
    pub fn classify_point(&self, point: &Point2D) -> f64 {
        (point.y - self.start.y) * (self.end.x - self.start.x)
            - (point.x - self.start.x) * (self.end.y - self.start.y)
    }
    
    pub fn intersect(&self, other: &Line2D) -> Option<Point2D> {
        let a1 = self.end.y - self.start.y;
        let b1 = self.start.x - self.end.x;
        let c1 = a1 * self.start.x + b1 * self.start.y;

        let a2 = other.end.y - other.start.y;
        let b2 = other.start.x - other.end.x;
        let c2 = a2 * other.start.x + b2 * other.start.y;

        let det = a1 * b2 - a2 * b1;
        if det.abs() < EPSILON { // Lines are parallel (or coincident)
            return None;
        }

        let x = (b2 * c1 - b1 * c2) / det;
        let y = (a1 * c2 - a2 * c1) / det;

        // Check if the intersection point lies on both segments
        if x < self.start.x.min(self.end.x) - EPSILON || x > self.start.x.max(self.end.x) + EPSILON ||
           y < self.start.y.min(self.end.y) - EPSILON || y > self.start.y.max(self.end.y) + EPSILON {
            return None;
        }
        if x < other.start.x.min(other.end.x) - EPSILON || x > other.start.x.max(other.end.x) + EPSILON ||
           y < other.start.y.min(other.end.y) - EPSILON || y > other.start.y.max(other.end.y) + EPSILON {
            return None
        }


        Some(Point2D::new(x, y))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BoundingBox {
    pub fn new_empty() -> Self {
        BoundingBox {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }

    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        BoundingBox {min_x, min_y, max_x, max_y}
    }

    pub fn expand_point(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    pub fn combine(&mut self, other: &BoundingBox) {
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);
    }

    pub fn from_segs(segs: &[Arc<Seg>]) -> Self {
        let mut bbox = BoundingBox::new_empty();
        for seg in segs{
            bbox.expand_point(seg.start.x, seg.start.y);
            bbox.expand_point(seg.end.x, seg.end.y);
        }
        bbox
    }

    // Check if the bounding box contains a point
    pub fn contains_point(&self, x: f64, y:f64) -> bool{
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    // Checks if two Bounding Boxes intersects
    pub fn intersects(&self, other: &BoundingBox) -> bool{
        self.max_x >= other.min_x && self.min_x <= other.max_x &&
        self.max_y >= other.min_y && self.min_y <= other.max_y
    }
}


// This function clips a line segment to a bounding box. Cohen-Sutherland Algorithm
pub fn clip_line(x1: &mut i32, y1: &mut i32, x2: &mut i32, y2: &mut i32, bounds: &BoundingBox) {
    // Cohen-Sutherland region codes for the endpoints
    let mut code1 = compute_out_code(*x1, *y1, bounds);
    let mut code2 = compute_out_code(*x2, *y2, bounds);
    let mut accept = false;

    loop {
        if code1 == 0 && code2 == 0 {
            // Both endpoints inside the box
            accept = true;
            break;
        } else if (code1 & code2) != 0 {
            // Both endpoints have a common outside region
            break;
        } else {
            // Some part of the line may be inside, clip
            let mut code_out = if code1 != 0 { code1 } else { code2 };

            let mut x = 0;
            let mut y = 0;
            let xmin = bounds.min_x as i32;
            let ymin = bounds.min_y as i32;
            let xmax = bounds.max_x as i32;
            let ymax = bounds.max_y as i32;

            // Find intersection point
            if (code_out & TOP) != 0 {
                x = *x1 + (*x2 - *x1) * (ymax - *y1) / (*y2 - *y1);
                y = ymax;
            } else if (code_out & BOTTOM) != 0 {
                x = *x1 + (*x2 - *x1) * (ymin - *y1) / (*y2 - *y1);
                y = ymin;
            } else if (code_out & RIGHT) != 0 {
                y = *y1 + (*y2 - *y1) * (xmax - *x1) / (*x2 - *x1);
                x = xmax;
            } else if (code_out & LEFT) != 0 {
                y = *y1 + (*y2 - *y1) * (xmin - *x1) / (*x2 - *x1);
                x = xmin;
            }

            // Replace the outside point with the intersection point
            if code_out == code1 {
                *x1 = x;
                *y1 = y;
                code1 = compute_out_code(*x1, *y1, bounds);
            } else {
                *x2 = x;
                *y2 = y;
                code2 = compute_out_code(*x2, *y2, bounds);
            }
        }
    }

    if accept {
        //println!("Line accepted: ({}, {}) to ({}, {})", x1, y1, x2, y2);
    } else {
        //println!("Line rejected");
    }

}

const INSIDE: i32 = 0; // 0000
const LEFT: i32   = 1; // 0001
const RIGHT: i32  = 2; // 0010
const BOTTOM: i32 = 4; // 0100
const TOP: i32    = 8; // 1000

// Compute the bit code for a point (x, y) using the clip rectangle
fn compute_out_code(x: i32, y: i32, bounds: &BoundingBox) -> i32 {
    let mut code = INSIDE; // initialised as being inside of clip window

    let xmin = bounds.min_x as i32;
    let ymin = bounds.min_y as i32;
    let xmax = bounds.max_x as i32;
    let ymax = bounds.max_y as i32;

    if x < xmin {           // to the left of clip window
        code |= LEFT;
    } else if x > xmax {      // to the right of clip window
        code |= RIGHT;
    }
    if y < ymin {           // below the clip window
        code |= BOTTOM;
    } else if y > ymax {      // above the clip window
        code |= TOP;
    }

    code
}