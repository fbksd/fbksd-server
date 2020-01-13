CREATE TABLE techniques (
    id INTEGER PRIMARY KEY,
    name VARCHAR NOT NULL,
    technique_type VARCHAR NOT NULL
)

CREATE TABLE workspaces (
    id SERIAL PRIMARY KEY,
    uuid VARCHAR(36) NOT NULL,
    commit_sha VARCHAR NOT NULL,
    creation_time TEXT NOT NULL,
    technique_id INTEGER,
    FOREIGN KEY(technique_id) REFERENCES techniques(id)
)

CREATE TABLE finished_workspaces (
    id SERIAL PRIMARY KEY,
    workspace_id INTEGER,
    finish_time TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
)

CREATE TABLE published_workspaces (
    id SERIAL PRIMARY KEY,
    workspace_id INTEGER,
    finish_time TEXT NOT NULL,
    publication_time TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
)
