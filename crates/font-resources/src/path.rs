use crate::{invalid, ExactPoint, ExpandedOutline, Result};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathCommand {
    MoveTo(ExactPoint),
    LineTo(ExactPoint),
    QuadTo {
        control: ExactPoint,
        end: ExactPoint,
    },
    Close,
}
pub struct QuadraticPath {
    commands: std::vec::IntoIter<PathCommand>,
}
impl Iterator for QuadraticPath {
    type Item = PathCommand;
    fn next(&mut self) -> Option<Self::Item> {
        self.commands.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.commands.size_hint()
    }
}
impl ExactSizeIterator for QuadraticPath {}
impl std::iter::FusedIterator for QuadraticPath {}
fn midpoint(a: ExactPoint, b: ExactPoint) -> Result<ExactPoint> {
    Ok(ExactPoint {
        x: a.x.midpoint(b.x)?,
        y: a.y.midpoint(b.y)?,
        on_curve: true,
    })
}
impl ExpandedOutline {
    /// Deterministic unhinted quadratic contour path. Implied points are exact midpoints.
    pub fn quadratic_path(&self) -> Result<QuadraticPath> {
        if self.points.len() > 1_000_000 || self.contour_ends.len() > self.points.len() {
            return Err(invalid("path point/contour budget"));
        }
        let mut commands = Vec::new();
        let mut begin = 0;
        for &end in &self.contour_ends {
            let end = end as usize;
            if end < begin || end >= self.points.len() {
                return Err(invalid("path contour endpoint"));
            }
            let points = &self.points[begin..=end];
            let first = points[0];
            let last = points[points.len() - 1];
            let (start, body) = if first.on_curve {
                (first, &points[1..])
            } else if last.on_curve {
                (last, &points[..points.len() - 1])
            } else {
                (midpoint(last, first)?, points)
            };
            commands.push(PathCommand::MoveTo(start));
            let mut pending = None;
            for &point in body {
                if point.on_curve {
                    if let Some(control) = pending.take() {
                        commands.push(PathCommand::QuadTo {
                            control,
                            end: point,
                        });
                    } else {
                        commands.push(PathCommand::LineTo(point));
                    }
                } else {
                    if let Some(control) = pending {
                        commands.push(PathCommand::QuadTo {
                            control,
                            end: midpoint(control, point)?,
                        });
                    }
                    pending = Some(point);
                }
            }
            if let Some(control) = pending {
                commands.push(PathCommand::QuadTo {
                    control,
                    end: start,
                });
            }
            commands.push(PathCommand::Close);
            begin = end + 1;
        }
        if begin != self.points.len() {
            return Err(invalid("unassigned path points"));
        }
        Ok(QuadraticPath {
            commands: commands.into_iter(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Coordinate;
    fn point(x: i32, on_curve: bool) -> ExactPoint {
        ExactPoint {
            x: Coordinate::from_integer(x),
            y: Coordinate::from_integer(0),
            on_curve,
        }
    }
    fn outline(points: Vec<ExactPoint>, ends: Vec<u32>) -> ExpandedOutline {
        ExpandedOutline {
            font_id: "test".into(),
            font_sha256: "0".repeat(64),
            face_index: 0,
            glyph_id: 2,
            points,
            contour_ends: ends,
            instances: vec![],
        }
    }
    #[test]
    fn oncurve_closed_lines() {
        let path = outline(vec![point(0, true), point(2, true)], vec![1])
            .quadratic_path()
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(
            path,
            vec![
                PathCommand::MoveTo(point(0, true)),
                PathCommand::LineTo(point(2, true)),
                PathCommand::Close
            ]
        );
    }
    #[test]
    fn all_offcurve_implied_midpoints_exact() {
        let path = outline(vec![point(0, false), point(1, false)], vec![1])
            .quadratic_path()
            .unwrap()
            .collect::<Vec<_>>();
        let mid = midpoint(point(0, false), point(1, false)).unwrap();
        assert_eq!(mid.x.numerator(), 1);
        assert_eq!(mid.x.shift(), 1);
        assert_eq!(
            path,
            vec![
                PathCommand::MoveTo(mid),
                PathCommand::QuadTo {
                    control: point(0, false),
                    end: mid
                },
                PathCommand::QuadTo {
                    control: point(1, false),
                    end: mid
                },
                PathCommand::Close
            ]
        );
    }
    #[test]
    fn offcurve_first_uses_last_oncurve() {
        let path = outline(vec![point(0, false), point(1, true)], vec![1])
            .quadratic_path()
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(
            path,
            vec![
                PathCommand::MoveTo(point(1, true)),
                PathCommand::QuadTo {
                    control: point(0, false),
                    end: point(1, true)
                },
                PathCommand::Close
            ]
        );
    }
    #[test]
    fn contours_and_invalid_endpoints() {
        let o = outline(vec![point(0, true), point(1, true)], vec![0, 1]);
        assert_eq!(o.quadratic_path().unwrap().len(), 4);
        for ends in [vec![], vec![2], vec![1, 1]] {
            assert!(outline(o.points.clone(), ends).quadratic_path().is_err());
        }
    }
}
