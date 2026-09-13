-- Rooms that give a resource back.
--
-- A bank is not a road. It goes nowhere: you walk in, you walk out the way you
-- came, and the only thing that changed is what you are carrying. That makes it
-- the wrong shape for `connectors`, whose whole subject is getting from one
-- room to a different one.
--
-- It is the right shape for routing, though, and that is the point. With
-- ferries priced at 10,000 to 35,000 silver, a character who cannot afford one
-- has three answers and a router should weigh all three at once:
--
--   1. go around          -- sometimes possible, sometimes not
--   2. pay and go         -- if the silver is already on them
--   3. via the bank       -- withdraw, then continue
--
-- (1) and (2) fall out of any search that knows the price. (3) only does if
-- the bank is something the search can *relax through*, rather than something
-- bolted on after a plan has failed -- which is what `go2` does, and why it
-- cannot tell you that walking would have been quicker than the detour plus
-- the fare. A bank the search knows about competes: a route that passes one on
-- the way gets the withdrawal for almost nothing.
--
-- # Which bank, and therefore which balance
--
-- `BANK ACCOUNT` reports a balance per *institution*, from anywhere:
--
--   You currently have the following amounts on deposit:
--        First Elanith Secured Bank: 627,283
--           United City-States Bank: 67
--                             Total: 627,350
--
-- So the balance is knowable while planning, and it decides *between* banks
-- rather than merely confirming one. A character with 627,283 in the Landing
-- and nothing in Icemule has a road through one bank and not the other, and a
-- router that cannot tell them apart picks wrong half the time.
--
-- `account` is what connects the two: the institution this room belongs to,
-- spelled as `BANK ACCOUNT` spells it. Many rooms to one account -- branches
-- share a balance, which is why the game reports by institution and not by
-- town.
--
-- # Why this is observed and not inferred
--
-- Pinefar and Icemule share an account. `[Pinefar, Depository]` and
-- `[Icemule Trace, Bank]` have no word in common, so no amount of reading room
-- titles would ever group them -- and grouping them wrongly is worse than not
-- grouping them, because it would plan a withdrawal against money in another
-- institution entirely.
--
-- Geography does not settle it either. Pinefar and Icemule are both north, and
-- so is Ta'Illistim; nothing says where an institution's edge falls. The map
-- database carries none of these names at all.
--
-- So `account` is filled in from watching a character, and left empty until it
-- is. Empty means unknown, and unknown plans optimistically -- the money is
-- assumed to be there. That is a choice rather than an omission: the failure
-- mode is the mildest in the system, a character standing in a bank, which is
-- indoors, safe, in a town, and one re-plan from somewhere else. The
-- alternative failure is refusing to plan because the money might not be
-- there, which strands the same character somewhere they actually were.
CREATE TABLE replenishers (
    room_uid INTEGER NOT NULL REFERENCES rooms(uid) ON DELETE CASCADE,
    -- What it gives back. Same open vocabulary as `connector_costs`.
    resource TEXT    NOT NULL,
    -- What to send. `{amount}` is the shortfall, which only the router knows:
    -- it is the resource this route has spent by the time it reaches here.
    --
    -- A third placeholder, and it earns its place the way the schema says one
    -- must -- on its own merits. `{item}` and `{portal}` stand for ids nobody
    -- can write down in advance; this stands for a number nobody can either,
    -- because it is a property of the route rather than of the room.
    command  TEXT    NOT NULL,
    -- The smallest the teller will hand over. Pinefar's banker refuses less
    -- than twenty, which `go2` works around by asking for at least that much.
    minimum  INTEGER NOT NULL DEFAULT 0,
    -- How long the errand takes, door to door. Time the router charges for
    -- the detour, so a bank two rooms away beats one across town.
    time_ms  INTEGER NOT NULL DEFAULT 0,
    -- The institution holding the balance, spelled as `BANK ACCOUNT` spells
    -- it. Empty means nobody has looked yet. Many rooms to one account.
    account  TEXT    NOT NULL DEFAULT '',
    -- When this room is not available at all. The Isle of Four Winds is a
    -- Premium subscription rather than a place, and routing a Standard
    -- subscriber to its bank plans a journey that ends at a refusal -- worse
    -- than no route, because the character has already travelled.
    --
    -- NULL for the ordinary case. A condition that forbids makes the room
    -- invisible to the search rather than merely expensive.
    condition_id TEXT REFERENCES conditions(id) ON DELETE CASCADE,
    PRIMARY KEY (room_uid, resource)
) WITHOUT ROWID;
