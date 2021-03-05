-- Add migration script here
CREATE TABLE IF NOT EXISTS crafts (
	id uuid PRIMARY KEY,
	until_finish float,
	total_cycles float,
	destroys_plant boolean,
	makes text
	);

CREATE TYPE seed_grower AS (
	id text,
	generations numeric
);

CREATE TABLE IF NOT EXISTS plants (
	id uuid PRIMARY KEY,
	xp numeric,
	until_yield float,
	craft references crafts(id),
	pedigree []seed_grower,
	archetype_handle text,
	on_market boolean
	);
	
CREATE TABLE IF NOT EXISTS tiles (
	id uuid PRIMARY KEY,
	acquired timestamp,
	plant references plants(id),
	steader text
	);

CREATE TABLE IF NOT EXISTS profiles (
	joined timestamp,
	last_active timestamp,
	last_farm timestamp,
	id text,
	xp numeric
	);


CREATE TABLE IF NOT EXISTS hacksteaders (
	user_id text,
	profile references profiles(id)
	);

CREATE TABLE IF NOT EXISTS tiles_steaders (
	steader_id text references hacksteaders(user_id),
	tile_id uuid references tiles(id)
	);

CREATE TABLE IF NOT EXISTS possess_steaders (
	steader_id text references hacksteaders(user_id),
	
