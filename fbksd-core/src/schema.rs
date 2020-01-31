table! {
    message_tasks (id) {
        id -> Integer,
        to_address -> Text,
        subject -> Text,
        text -> Text,
    }
}

table! {
    normal_tasks (id) {
        id -> Integer,
        technique_id -> Integer,
        commit_sha -> Text,
        docker_img -> Text,
        task_type -> Integer,
        task_data -> Nullable<Binary>,
    }
}

table! {
    priority_tasks (id) {
        id -> Integer,
        technique_id -> Integer,
        commit_sha -> Text,
        docker_img -> Text,
        task_type -> Integer,
        task_data -> Nullable<Binary>,
    }
}

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

joinable!(normal_tasks -> techniques (technique_id));
joinable!(priority_tasks -> techniques (technique_id));
joinable!(workspaces -> techniques (technique_id));

allow_tables_to_appear_in_same_query!(
    message_tasks,
    normal_tasks,
    priority_tasks,
    techniques,
    workspaces,
);
