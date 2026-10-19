//! Typed caller continuations share the worker slot with ordinary route requests.
use super::*;
use ::navigation::worker::{Output, Query as WorkerQuery, Request};

pub(crate) type QueryResult = Result<Option<Output>, String>;

pub(super) struct PendingQuery {
    pub(super) id: u64,
    pub(super) map_id: u32,
    pub(super) goal: Goal,
    query: WorkerQuery,
    pub(super) result: Option<QueryResult>,
}

impl Navigation {
    /// None is pending, not a clear probe, an unbaked map, or a failed query.
    pub(crate) fn query_deferred(
        &mut self,
        entity: Entity,
        map_id: u32,
        goal: Goal,
        query: WorkerQuery,
    ) -> Option<QueryResult> {
        if let Some(previous) = self.queries.get(&entity)
            && previous.map_id == map_id
            && previous.goal == goal
            && same_query(&previous.query, &query)
        {
            return previous.result.clone();
        }
        // A probe replaces an unfinished plan, never the route's current progress.
        self.pending.remove(&entity);
        let id = self.allocate_id();
        self.queries.insert(
            entity,
            PendingQuery {
                id,
                map_id,
                goal,
                query: query.clone(),
                result: None,
            },
        );
        self.worker.submit(Request {
            entity: entity.to_bits(),
            id,
            map_id,
            query,
        });
        None
    }

    pub(crate) fn query_owner(&self, entity: Entity) -> Option<Goal> {
        self.queries.get(&entity).map(|query| query.goal)
    }

    pub(super) fn query_pending(&self, entity: Entity) -> bool {
        self.queries
            .get(&entity)
            .is_some_and(|query| query.result.is_none())
    }

    /// Only the owning continuation can cancel its probe; chase cancellation is separate.
    pub(crate) fn cancel_query(&mut self, entity: Entity, goal: Goal) {
        if self
            .queries
            .get(&entity)
            .is_some_and(|query| query.goal == goal)
        {
            let query = self.queries.remove(&entity).expect("matched query");
            if query.result.is_none() {
                self.worker.cancel(entity.to_bits());
            }
        }
    }

    pub(super) fn publish_query(&mut self, response: &::navigation::worker::Response) -> bool {
        let entity = Entity::from_bits(response.entity);
        let Some(query) = self.queries.get_mut(&entity) else {
            return false;
        };
        if query.id != response.id {
            return false;
        }
        query.result = Some(response.result.clone());
        true
    }

    pub(super) fn allocate_id(&mut self) -> u64 {
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("navigation request ID exhausted");
        self.next_id
    }
}

pub(super) fn query_from(query: &PendingQuery) -> Vec3 {
    match &query.query {
        WorkerQuery::Path { from, .. }
        | WorkerQuery::SmoothPath { from, .. }
        | WorkerQuery::Collision { from, .. }
        | WorkerQuery::Candidates { from, .. } => *from,
    }
}

fn same_query(previous: &WorkerQuery, next: &WorkerQuery) -> bool {
    match (previous, next) {
        (WorkerQuery::Path { from: a, to: b }, WorkerQuery::Path { from: c, to: d }) => {
            horizontal(*a - *c) <= TELEPORT_DISTANCE && b.distance(*d) <= REPATH_DISTANCE
        }
        (
            WorkerQuery::Candidates {
                from: a,
                targets: b,
            },
            WorkerQuery::Candidates {
                from: c,
                targets: d,
            },
        ) => horizontal(*a - *c) <= TELEPORT_DISTANCE && b == d,
        // Fear and script generators hold their feet while these captured stages run.
        _ => previous == next,
    }
}

pub(super) fn prune_queries(
    nav: &mut Navigation,
    actors: &Query<(Option<&crate::maps::InMap>, Option<&Position>)>,
) {
    let obsolete: Vec<_> = nav
        .queries
        .iter()
        .filter_map(|(entity, query)| {
            let Ok((map, at)) = actors.get(*entity) else {
                return Some((*entity, query.goal));
            };
            let wrong_map = map.is_none_or(|map| map.map_id != query.map_id);
            let moved =
                at.is_none_or(|at| horizontal(point(*at) - query_from(query)) > TELEPORT_DISTANCE);
            (wrong_map || moved).then_some((*entity, query.goal))
        })
        .collect();
    for (entity, goal) in obsolete {
        nav.cancel_query(entity, goal);
    }
}
