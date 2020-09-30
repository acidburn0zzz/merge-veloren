-- Back up the character table since we need to drop it in order to recreate
-- the stats table which character has an FK to
CREATE TEMP TABLE _character
(
    character_id INT NOT NULL
        PRIMARY KEY,
    player_uuid TEXT NOT NULL,
    alias TEXT NOT NULL
);

INSERT
INTO    _character
SELECT  character_id,
        player_uuid,
        alias
FROM    character;

DROP TABLE character;

-- Recreate the stats table with the new inv_slots column and insert the old data back into it
ALTER TABLE stats RENAME TO _stats;

CREATE TABLE stats
(
    stats_id INT NOT NULL
        PRIMARY KEY
        REFERENCES entity,
    level INT NOT NULL,
    exp INT NOT NULL,
    inv_slots INT NOT NULL,
    endurance INT NOT NULL,
    fitness INT NOT NULL,
    willpower INT NOT NULL
);

INSERT INTO stats
SELECT  stats_id,
        level,
        exp,
        36 as inv_slots,
        endurance,
        fitness,
        willpower
FROM    _stats;

DROP TABLE _stats;

-- Re-create the original character table and insert the data from the backup to it
CREATE TABLE character
(
    character_id INT NOT NULL
        PRIMARY KEY
        REFERENCES body
        REFERENCES item
        REFERENCES stats,
    player_uuid TEXT NOT NULL,
    alias TEXT NOT NULL
);

CREATE INDEX idx_player_uuid
    ON character (player_uuid);

INSERT
INTO    character
SELECT  character_id,
        player_uuid,
        alias
FROM    _character;