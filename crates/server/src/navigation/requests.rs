//! Fixed-tick route requests and publication; the worker never owns route progress.
use super::*;
use ::navigation::worker::{Output, Query as WorkerQuery, Request as WorkerRequest};

pub(super) struct PendingRoute {
    id: u64,
    pub(super) request: MoveRequest,
    pub(super) done: bool,
}

impl Navigation {
    pub(crate) fn route_pending(&self, entity: Entity) -> bool {
        self.query_pending(entity)
            || self
                .pending
                .get(&entity)
                .is_some_and(|pending| !pending.done)
    }

    pub(super) fn request_route(&mut self, entity: Entity, request: MoveRequest) {
        if self.query_pending(entity) {
            return;
        }
        if self
            .pending
            .get(&entity)
            .is_some_and(|pending| same_route_request(&pending.request, &request))
        {
            return;
        }
        let id = self.allocate_id();
        self.pending.insert(
            entity,
            PendingRoute {
                id,
                request,
                done: false,
            },
        );
        self.worker.submit(WorkerRequest {
            entity: entity.to_bits(),
            id,
            map_id: request.map_id,
            query: WorkerQuery::Path {
                from: point(request.from),
                to: point(request.to),
            },
        });
    }

    pub(super) fn publish_ready_routes(&mut self) {
        #[cfg(test)]
        self.applied.clear();
        for response in self.worker.poll() {
            let entity = Entity::from_bits(response.entity);
            if self.publish_query(&response) {
                #[cfg(test)]
                self.applied.push((entity, response.id));
                continue;
            }
            let Some(pending) = self.pending.get_mut(&entity) else {
                continue;
            };
            if pending.id != response.id {
                continue;
            }
            pending.done = true;
            let request = pending.request;
            #[cfg(test)]
            self.applied.push((entity, response.id));
            self.publish_route(entity, request, response.result);
        }
    }

    fn publish_route(
        &mut self,
        entity: Entity,
        request: MoveRequest,
        result: Result<Option<Output>, String>,
    ) {
        let points = match result {
            Ok(None) => {
                if self.unbaked.insert(request.map_id) {
                    let dir = self.dir.join(request.map_id.to_string());
                    error!(
                        "Map {} has no navigation bake in {}; no baked route available",
                        request.map_id,
                        dir.display()
                    );
                }
                self.routes.remove(&entity);
                return;
            }
            Ok(Some(Output::Path(points))) => Ok(Corners::new(points)),
            Ok(Some(_)) => Err("navigation route returned a non-path result".into()),
            Err(error) => Err(error),
        };
        if let Err(error) = &points {
            error!(
                "Creature {entity} navigation {:?} on map {} failed: {error}; no direct movement",
                request.goal, request.map_id
            );
        }
        self.routes.insert(
            entity,
            Route {
                map_id: request.map_id,
                goal: request.goal,
                destination: point(request.to),
                last_target: point(request.to),
                expected_from: point(request.from),
                reach: request.reach,
                path: points,
            },
        );
    }

    pub(super) fn invalidate_moved_route(&mut self, entity: Entity, request: &MoveRequest) {
        let invalid = self.routes.get(&entity).is_some_and(|route| {
            route.map_id != request.map_id
                || horizontal(route.expected_from - point(request.from)) > TELEPORT_DISTANCE
        });
        if invalid {
            self.cancel_route(entity);
        }
    }

    pub(crate) fn continue_route(
        &mut self,
        entity: Entity,
        map_id: u32,
        from: Position,
        distance: f32,
    ) -> Step {
        let Some(route) = self.routes.get(&entity) else {
            return Step::Pending;
        };
        let request = MoveRequest {
            map_id,
            from,
            distance,
            goal: route.goal,
            to: position(route.last_target),
            reach: route.reach,
        };
        self.invalidate_moved_route(entity, &request);
        self.advance_published_route(entity, request, true)
    }

    pub(super) fn advance_published_route(
        &mut self,
        entity: Entity,
        request: MoveRequest,
        pending: bool,
    ) -> Step {
        let from = point(request.from);
        let Some(route) = self.routes.get_mut(&entity) else {
            return Step::Pending;
        };
        let Ok(corners) = &mut route.path else {
            return if pending {
                Step::Pending
            } else {
                Step::Blocked
            };
        };
        if !pending {
            route.last_target = point(request.to);
            route.reach = request.reach;
        }
        let (target, reach) = (route.last_target, route.reach);
        let (next, reached) = corners.advance(from, request.distance, reach, target);
        route.expected_from = next;
        if pending {
            if next == from {
                return Step::Pending;
            }
            // An old route may advance while new work is in flight. The latest
            // request must start at those real new feet, not pull them backward.
            self.request_route(
                entity,
                MoveRequest {
                    from: position(next),
                    ..request
                },
            );
        }
        Step::Moved {
            position: position(next),
            reached: reached && !pending,
        }
    }

    #[cfg(test)]
    pub(crate) fn drain_until_idle(&mut self) {
        while !self.worker.is_idle() {
            self.publish_ready_routes();
            std::thread::yield_now();
        }
    }
}

fn same_route_request(previous: &MoveRequest, next: &MoveRequest) -> bool {
    let same_target = if matches!(next.goal, Goal::Follow(_)) {
        point(previous.to) == point(next.to)
    } else {
        point(previous.to).distance(point(next.to)) <= REPATH_DISTANCE
    };
    previous.map_id == next.map_id
        && previous.goal == next.goal
        && point(previous.from) == point(next.from)
        && same_target
}

pub(super) fn publish_navigation_results(mut nav: ResMut<Navigation>) {
    nav.publish_ready_routes();
}
