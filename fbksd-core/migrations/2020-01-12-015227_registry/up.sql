CREATE TABLE techniques (
    id INTEGER PRIMARY KEY NOT NULL,
    technique_type INTEGER NOT NULL,
    short_name VARCHAR UNIQUE NOT NULL,
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