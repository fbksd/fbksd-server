table! {
    techniques (id) {
        id -> Integer,
        technique_type -> Integer,
        short_name -> Text,
        full_name -> Text,
        citation -> Text,
        comment -> Text,
        num_workspaces -> Integer,
    }
}

table! {
    workspaces (id) {
        id -> Integer,
        technique_id -> Integer,
        uuid -> Text,
        commit_sha -> Text,
        docker_image -> Text,
        status -> Integer,
        creation_time -> Timestamp,
        finish_time -> Nullable<Timestamp>,
        publication_time -> Nullable<Timestamp>,
    }
}

joinable!(workspaces -> techniques (technique_id));

allow_tables_to_appear_in_same_query!(techniques, workspaces,);
