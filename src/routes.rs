use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post},
    Router,
};

use crate::handlers::{
    admin, fuel, gas_stations, geocode, location, posts, reels, rooms, rust_practice, trips,
    waypoints,
};
use crate::AppState;

pub fn api() -> Router<AppState> {
    let reels_routes = Router::new()
        .route("/api/reels", get(reels::list).post(reels::upload))
        .route("/api/reels/:id/video", get(reels::serve_video))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)); // 100MB for video upload

    let posts_routes = Router::new()
        .route("/api/posts", get(posts::list).post(posts::create))
        .route("/api/posts/:id/image", get(posts::serve_image))
        .route("/api/posts/:id/comments", post(posts::create_comment))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)); // 10MB for image

    Router::new()
        // Rooms
        .route("/api/rooms", get(rooms::list_my_rooms).post(rooms::create))
        .route("/api/rooms/join", post(rooms::join_by_code))
        // Locations (with speed + heading)
        .route("/api/locations", post(location::report))
        .route("/api/locations", get(location::list))
        // Trips & waypoints
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
        // Fuel
        .route("/api/rooms/:room_id/fuel", post(fuel::create))
        .route("/api/rooms/:room_id/fuel", get(fuel::list))
        .route("/api/fuel", get(fuel::list_all))
        // Gas stations
        .route("/api/gas-stations", get(gas_stations::list))
        // Geocoding proxy (Nominatim + Redis cache)
        .route("/api/geocode/search", get(geocode::search))
        // Admin — user management
        .route("/api/admin/users", get(admin::list_users))
        .route(
            "/api/admin/users/:id/role",
            patch(admin::update_role_handler),
        )
        // Misc
        .route("/api/rust-practice/generate", post(rust_practice::generate))
        .route("/api/ws/:room_id", get(crate::ws::handler))
        .merge(reels_routes)
        .merge(posts_routes)
}
