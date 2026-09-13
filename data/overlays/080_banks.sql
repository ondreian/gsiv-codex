-- The banks, as rooms that give silver back.
--
-- Derived from the `bank` facet rather than listed by hand: the facets come
-- from the map database's own tags, so a bank that gets added there becomes
-- routable here without anybody noticing it had to.
--
-- `withdraw {amount} silvers` is what every teller in the game takes, with one
-- exception recorded below. `go2` sends the same string.
--
-- 30 seconds for the errand: walking to the window, the withdrawal, and
-- walking back out. It is an estimate, not a measurement, and it is deliberately
-- not small -- a router that thinks a bank trip is free will route through one
-- to save a step.
INSERT INTO replenishers(room_uid, resource, command, minimum, time_ms)
SELECT room_uid, 'silver', 'withdraw {amount} silvers', 0, 30000
  FROM room_facets
 WHERE type = 'bank';

-- Pinefar's banker is not a teller and does not take `withdraw`. `go2` has
-- carried this special case for years:
--
--   if XMLData.room_title == '[Pinefar, Depository]'
--     fput "ask banker for #{[(needed_silvers - silver), 20].max} silvers"
--
-- The twenty is a floor, not a fee: he will not be bothered for less.
UPDATE replenishers
   SET command = 'ask banker for {amount} silvers',
       minimum = 20
 WHERE resource = 'silver'
   AND room_uid = 4564009;

-- # Which institution holds the balance
--
-- Observed by standing in each bank, not inferred. Two attempts to infer had
-- already failed by the time the first one was confirmed:
--
--   the Landing's lobby calls it "the First Elanith Secure Stronghold";
--     BANK ACCOUNT calls it "First Elanith Secured Bank"
--   Icemule's teller room bears "the watchfire emblem of Northwatch";
--     the account is "Icemule Trace Bank", and Northwatch is a different bank
--     entirely
--
-- The method is one command: DEPOSIT 1 SILVER, then BANK ACCOUNT, and read
-- which line moved. CHECK BALANCE works at most tellers and is read-only.
UPDATE replenishers SET account = 'First Elanith Secured Bank'
 WHERE resource = 'silver' AND room_uid = 2005;        -- [First Elanith Bank, Teller]

UPDATE replenishers SET account = 'Icemule Trace Bank'
 WHERE resource = 'silver' AND room_uid = 4043301;     -- [Icemule Trace, Bank]

-- Pinefar is not a bank. The banker says so himself:
--
--   "I can take ye silvers to the 'Mule and put em in ye bank account.  If ye
--    be interested, jus' GIVE me ye silvers or a note from the 'Mule.  A few
--    folks also done talked me into givin' ye credited withdrawls too.  Ye kin
--    ASK me FOR some SILVERS if ye wants a withdrawl, but I be takin' a bit
--    for my troubles when I gets to the bank."
--
-- So he is a courier to Icemule, drawing on the same account -- which is why
-- Pinefar and Icemule share a balance, and why no reading of the two room
-- titles would ever have grouped them.
--
-- He also charges. "I be takin' a bit for my troubles" is a surcharge this
-- schema cannot yet express: asking for N silvers here does not hand you N.
-- Until it can, a route through Pinefar will come up slightly short, which the
-- walker discovers and re-plans around.
--
-- The twenty-silver floor is his, not the bank's: asked for one, he says
-- "I'd have to charge ye more than that to make it worth my while."
UPDATE replenishers SET account = 'Icemule Trace Bank'
 WHERE resource = 'silver' AND room_uid = 4564009;     -- [Pinefar, Depository]

-- Swept 2026-09-13, one town at a time: DEPOSIT 1 SILVER, then BANK ACCOUNT,
-- and read which line moved. Every one of these is a name no heuristic would
-- have produced -- `[River's Rest Bank, Teller]` belongs to the Bank of Torre
-- County, and `[Bank of Zul Logoth]` to the Bank of Kharag 'doth Dzulthu.
UPDATE replenishers SET account = 'Vornavis Bank of Solhaven'
 WHERE resource = 'silver' AND room_uid = 4740018;     -- [Bank of Vornavis, Solhaven]

UPDATE replenishers SET account = 'Bank of Torre County'
 WHERE resource = 'silver' AND room_uid = 2101904;     -- [River's Rest Bank, Teller]

UPDATE replenishers SET account = 'Great Bank of Kharam-Dzu'
 WHERE resource = 'silver' AND room_uid = 3003042;     -- [The Bank of Kharam-Dzu]

UPDATE replenishers SET account = 'Bank of Kharag ''doth Dzulthu'
 WHERE resource = 'silver' AND room_uid = 13010001;    -- [Bank of Zul Logoth]

UPDATE replenishers SET account = 'Kraken''s Fall Bank'
 WHERE resource = 'silver' AND room_uid = 7118401;     -- [Kraken's Fall Bank, Teller]

UPDATE replenishers SET account = 'Cysaegir Bank'
 WHERE resource = 'silver' AND room_uid = 14051015;    -- [Cysaegir Bank]

-- The United Bank of City-States is the one with branches, and the reason this
-- is keyed by institution rather than by town: Ta'Illistim and Ta'Vaalor draw
-- on one balance. Watched moving 67 -> 68 in Illistim and 68 -> 69 in Vaalor.
UPDATE replenishers SET account = 'United City-States Bank'
 WHERE resource = 'silver' AND room_uid IN (13103004, 14106002);

-- Clenchfist Bros. Banking is in the Northern Caravansary and draws on the
-- Landing's bank -- a `474xxxx` uid, next door to Solhaven's in the same
-- block, on First Elanith's books. Watched moving 627,185 -> 627,190 on a
-- five-silver deposit. One more reason none of this is inferable from where
-- a room sits.
UPDATE replenishers SET account = 'First Elanith Secured Bank'
 WHERE resource = 'silver' AND room_uid = 4746114;

-- # The Isle of Four Winds has no road, so it has no gate either
--
-- `[Four Winds Bank, Teller Windows]` cannot be reached and is left unmapped.
-- Two separate reasons, and it is worth writing both down because the second
-- outlives the first:
--
--   1. Premium. `PREMIUM START` on a Standard account answers "You are not
--      currently a Premium subscriber." Mist Harbor is a subscription tier
--      rather than a place.
--
--   2. There is no published edge onto the island at all. Thirteen town
--      squares reach room 3668 -- Wehnimer's, Solhaven, Icemule, Kharam-Dzu,
--      Ta'Vaalor, Cysaegir, Zul Logoth, River's Rest, Kraken's Fall, Seareach,
--      Ta'Nalfein, Sailor's Grief, the Hinterwilds -- and every one is a `;e`
--      script edge, so the extractor dropped them.
--
-- The mechanism is a connector nobody has written, and it is not only Premium:
-- Ta'Illistim's copy reads `UserVars.mapdb_fwi_trinket`, so it wants an item in
-- hand as well. One anchor per town square, one destination, gated on the
-- trinket and the subscription -- the same shape as the periapt to the Sanctum.
--
-- No condition is published here. A `not-premium` condition hung on nothing
-- forbids nothing, which is exactly how the Voln gate shipped offering a
-- Voln-only road to everybody. It belongs with the connector, when there is
-- one.

-- # Observed, and deliberately not acted on
--
-- Walking to Brindlestoat's Moneylender (7503260) on 2026-09-13 got 167 steps
-- and then:
--
--   step 167: no-such-exit -- You can't go there.
--   travel failed: no-such-exit on "climb sliver", and no other route
--
-- The edge is 4132054 [Abbey, Teleportation Chamber] -> 7503253, and the map
-- database records it as a plain unconditional `climb sliver`.
--
-- No overlay is written for it, on purpose. One character being refused does
-- not establish that a road does not exist -- the Abbey is a way into the
-- Hinterwilds and the sliver is far more likely *gated* on something Norhaak
-- has not done than absent. Excluding it would invent "this road is not there"
-- exactly as publishing it ungated invented "anyone may take it".
--
-- What it needs is a character who can use it, to say what the gate is. Until
-- then the router handles it correctly on its own: the refusal classifies as
-- `no-such-exit`, `travel::avoid` drops the edge, and the journey re-plans --
-- which is the whole point of the failure vocabulary and cost nothing but the
-- walk.
