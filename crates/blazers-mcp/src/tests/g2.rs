use super::*;

#[test]
fn records_round_trip_through_the_real_database() {
    let (_dir, db) = test_db();
    let _collection = single(
        call_tool(13, "blazers_collections_create", json!({ "name": "posts" })),
        db.clone(),
    );

    let created = single(
        call_tool(
            14,
            "blazers_records_create",
            json!({
                "collection": "posts", "id": "post-1", "data": { "title": "Hello" }
            }),
        ),
        db.clone(),
    );
    assert!(tool_text(&created).contains("Hello"));

    let fetched = single(
        call_tool(
            15,
            "blazers_records_get",
            json!({ "collection": "posts", "id": "post-1" }),
        ),
        db.clone(),
    );
    assert!(tool_text(&fetched).contains("Hello"));

    let updated = single(
        call_tool(
            16,
            "blazers_records_update",
            json!({
                "collection": "posts", "id": "post-1", "data": { "title": "Updated" }
            }),
        ),
        db.clone(),
    );
    assert!(tool_text(&updated).contains("Updated"));

    let listed = single(
        call_tool(17, "blazers_records_list", json!({ "collection": "posts" })),
        db.clone(),
    );
    assert!(
        tool_text(&listed).contains("Updated"),
        "update was not persisted"
    );

    let deleted = single(
        call_tool(
            18,
            "blazers_records_delete",
            json!({ "collection": "posts", "id": "post-1" }),
        ),
        db.clone(),
    );
    assert!(tool_text(&deleted).contains("\"deleted\": true"));

    let missing = single(
        call_tool(
            19,
            "blazers_records_get",
            json!({ "collection": "posts", "id": "post-1" }),
        ),
        db,
    );
    assert!(error_text(&missing).contains("not found"));
}

#[test]
fn records_create_generates_an_id_when_none_is_given() {
    let (_dir, db) = test_db();
    let _collection = single(
        call_tool(20, "blazers_collections_create", json!({ "name": "auto" })),
        db.clone(),
    );
    let created = single(
        call_tool(
            21,
            "blazers_records_create",
            json!({ "collection": "auto", "data": { "n": 1 } }),
        ),
        db,
    );
    assert!(
        tool_text(&created).contains("\"id\""),
        "no id was generated"
    );
}

#[test]
fn records_list_rejects_a_per_page_above_the_cap() {
    let (_dir, db) = test_db();
    let _collection = single(
        call_tool(
            22,
            "blazers_collections_create",
            json!({ "name": "capped" }),
        ),
        db.clone(),
    );
    let reply = single(
        call_tool(
            23,
            "blazers_records_list",
            json!({ "collection": "capped", "per_page": 100_000 }),
        ),
        db,
    );
    assert!(error_text(&reply).contains("per_page must be between"));
}

#[test]
fn records_list_rejects_a_page_below_one() {
    let (_dir, db) = test_db();
    let _collection = single(
        call_tool(24, "blazers_collections_create", json!({ "name": "paged" })),
        db.clone(),
    );
    let reply = single(
        call_tool(
            25,
            "blazers_records_list",
            json!({ "collection": "paged", "page": 0 }),
        ),
        db,
    );
    assert!(error_text(&reply).contains("page must be 1 or greater"));
}

#[test]
fn records_writes_enforce_the_collection_schema() {
    let (_dir, db) = test_db();
    let schema = json!({ "name": "typed", "schema": r#"{"title":"string","views?":"integer"}"# });
    let _collection = single(
        call_tool(26, "blazers_collections_create", schema.clone()),
        db.clone(),
    );

    let bad_create = single(
        call_tool(
            27,
            "blazers_records_create",
            json!({ "collection": "typed", "data": { "views": "many" } }),
        ),
        db.clone(),
    );
    assert!(error_text(&bad_create).contains("title"));

    let good = single(
        call_tool(
            28,
            "blazers_records_create",
            json!({ "collection": "typed", "id": "t1", "data": { "title": "Hi" } }),
        ),
        db.clone(),
    );
    assert!(tool_text(&good).contains("Hi"));

    let bad_update = single(
        call_tool(
            29,
            "blazers_records_update",
            json!({ "collection": "typed", "id": "t1", "data": { "title": 42 } }),
        ),
        db.clone(),
    );
    assert!(error_text(&bad_update).contains("string"));

    // collections_create rejects malformed schemas up front.
    let bad_schema = single(
        call_tool(
            30,
            "blazers_collections_create",
            json!({ "name": "bad", "schema": r#"{"x":"not-a-type"}"# }),
        ),
        db,
    );
    assert!(error_text(&bad_schema).contains("unknown type"));
}
