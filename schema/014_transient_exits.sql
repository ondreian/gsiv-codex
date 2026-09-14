-- Regions where an exit being absent does not mean it is not there.
--
-- "You can't go there." is the game's answer both when an exit does not exist
-- and when it has merely wandered off, and nothing in the sentence separates
-- them. That matters because the right responses are opposite: a missing exit
-- should be avoided and re-planned around, and a moved one should be waited
-- for and tried again.
--
-- Since the message cannot carry the difference, the world has to. In the Rift
-- the exits move by design -- doors, mirrors, threads, maws, staircases and
-- fissures wander the rings (see `circuits`) -- and the Hinterwilds slivers
-- teleport. Those are facts about places, so they are recorded per region
-- rather than per edge: a rule written for one door would have to be written
-- again for every door, and a new one would arrive unmarked.
--
-- What it changes, client-side: a refusal in one of these regions classifies
-- as `retry` rather than `edge-wrong`. Nothing is avoided, the walk pauses and
-- looks again, and the road survives a bad moment.
--
-- Being wrong here is cheap in one direction and not the other. Marking a
-- fixed region transient costs a few wasted retries before the walk gives up.
-- Leaving a transient one unmarked silently deletes working roads from every
-- route for the rest of the session, on the strength of one refusal.
CREATE TABLE transient_exits (
    -- A `room_sets` name. The set is the region.
    set_name TEXT PRIMARY KEY REFERENCES room_sets(name) ON DELETE CASCADE,
    -- Why, in the world's terms, so the next person does not have to guess
    -- whether it was measured or assumed.
    reason   TEXT NOT NULL DEFAULT ''
);
