CREATE TABLE users (
    id SERIAL PRIMARY KEY,

    username VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password VARCHAR(255) NOT NULL,

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE github_repos (
    id SERIAL PRIMARY KEY,
    
    installation_ids INTEGER[],

    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE projects (
    id SERIAL PRIMARY KEY,
    project_id VARCHAR(255) NOT NULL UNIQUE,

    install_cmds TEXT[],
    run_cmds TEXT[],
    build_cmds TEXT[],

    branch VARCHAR(255),
    project_type VARCHAR(255),
    full_name VARCHAR(255) NOT NULL,

    dist_dir VARCHAR(255) NOT NULL,
    root_dir VARCHAR(255) NOT NULL,

    url VARCHAR(255) UNIQUE,

    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    commit_hash VARCHAR(255),
    envs TEXT[],
    last_deployment_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(255) NOT NULL,

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE project_traffic (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    date DATE NOT NULL DEFAULT CURRENT_DATE,
    request_count INTEGER NOT NULL DEFAULT 1,
    UNIQUE(project_id, date)
);