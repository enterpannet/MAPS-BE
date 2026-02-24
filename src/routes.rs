use axum::{routing::{delete, get, post}, Router};

use crate::handlers::{fuel, gas_stations, location, rooms, trips, waypoints};
use crate::AppState;

pub fn api() -> Router<AppState> {
    Router::new()
        .route("/api/rooms", get(rooms::list_my_rooms).post(rooms::create))
        .route("/api/rooms/join", post(rooms::join_by_code))
        .route("/api/locations", post(location::report))
        .route("/api/locations", get(location::list))
        .route(
            "/api/rooms/:room_id/trips",
            get(trips::list).post(trips::create),
        )
        .route(
            "/api/rooms/:room_id/waypoints",
            get(waypoints::list).post(waypoints::create),
        )
        .route(
            "/api/rooms/:room_id/waypoints/:waypoint_id",
            delete(waypoints::delete),
        )
        .route("/api/rooms/:room_id/fuel", post(fuel::create))
        .route("/api/rooms/:room_id/fuel", get(fuel::list))
        .route("/api/fuel", get(fuel::list_all))
        .route("/api/gas-stations", get(gas_stations::list))
        .route("/api/ws/:room_id", get(crate::ws::handler))
}
