CREATE TABLE techniques (
    id INTEGER PRIMARY KEY NOT NULL,
    fbksd_token VARCHAR(36) NOT NULL,
    technique_type INTEGER NOT NULL,      -- 0: denoiser; 1: sampler
    short_name VARCHAR UNIQUE NOT NULL,   -- Acronym of the technique
    full_name VARCHAR NOT NULL,
    citation VARCHAR NOT NULL,
    comment VARCHAR NOT NULL,
    num_workspaces INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE workspaces (
    id INTEGER PRIMARY KEY NOT NULL,
    technique_id INTEGER NOT NULL,
    uuid VARCHAR(36) NOT NULL,
    commit_sha VARCHAR NOT NULL,
    docker_image VARCHAR NOT NULL,
    status INTEGER NOT NULL,
    creation_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finish_time DATETIME,
    publication_time DATETIME,
    FOREIGN KEY(technique_id) REFERENCES techniques(id)
);

-- used for running benchmarks
CREATE TABLE normal_tasks (
    id INTEGER PRIMARY KEY NOT NULL,
    technique_id INTEGER NOT NULL,
    commit_sha VARCHAR NOT NULL,
    docker_img VARCHAR NOT NULL,
    task_type INTEGER NOT NULL,
    task_data BLOB,
    FOREIGN KEY(technique_id) REFERENCES techniques(id)
);

-- used for build and publish tasks
-- workers will consume all tasks in this queue before start consuming from `normal_queue`
CREATE TABLE priority_tasks (
    id INTEGER PRIMARY KEY NOT NULL,
    technique_id INTEGER NOT NULL,
    commit_sha VARCHAR NOT NULL,
    docker_img VARCHAR NOT NULL,
    task_type INTEGER NOT NULL,
    task_data BLOB,
    FOREIGN KEY(technique_id) REFERENCES techniques(id)
);

CREATE TABLE message_tasks (
    id INTEGER PRIMARY KEY NOT NULL,
    to_address VARCHAR(256) NOT NULL,
    subject TEXT NOT NULL,
    text TEXT NOT NULL
);