//! Synchronous inspection for test fixtures only; still runs every query on the worker.
use super::*;
use ::navigation::worker::{Output, Query};

impl Navigation {
    fn drain_query(&mut self, map_id: u32, query: Query) -> Result<Option<Output>, String> {
        // Separate from real fixture actors; no ECS state or movement is attached here.
        let entity = Entity::from_bits(1_000_000_000);
        self.cancel_query(entity, Goal::Point);
        let _ = self.query_deferred(entity, map_id, Goal::Point, query.clone());
        self.drain_until_idle();
        let result = self
            .query_deferred(entity, map_id, Goal::Point, query)
            .expect("test drain publishes the current query");
        self.cancel_query(entity, Goal::Point);
        result
    }

    pub(crate) fn query_path(
        &mut self,
        map_id: u32,
        from: Vec3,
        to: Vec3,
    ) -> Result<Option<Vec<Vec3>>, String> {
        match self.drain_query(map_id, Query::Path { from, to })? {
            Some(Output::Path(points)) => Ok(Some(points)),
            None => Ok(None),
            Some(_) => Err("test path returned a non-path result".into()),
        }
    }

    pub(crate) fn first_collision(
        &mut self,
        map_id: u32,
        from: Vec3,
        to: Vec3,
    ) -> Result<Option<Vec3>, String> {
        match self.drain_query(map_id, Query::Collision { from, to })? {
            Some(Output::Collision(point)) => Ok(Some(point)),
            None => Ok(None),
            Some(_) => Err("test collision returned a non-collision result".into()),
        }
    }
}
