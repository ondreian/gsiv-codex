-- The facet vocabulary. Curated: that a `gemshop` tag means a `gem_dealer` is
-- a decision, not something the mapdb states.
--
-- `location` is not here; migration 002 creates it, because room sets cannot
-- be defined without it.
INSERT INTO facet_types(type, class, description) VALUES ('adventurers_guild', 'poi', 'Adventurer''s Guild (bounties)');
INSERT INTO facet_types(type, class, description) VALUES ('alchemist', 'poi', 'alchemy supplies');
INSERT INTO facet_types(type, class, description) VALUES ('bank', 'poi', 'currency exchange / bank teller');
INSERT INTO facet_types(type, class, description) VALUES ('forge', 'poi', 'smithy / forge');
INSERT INTO facet_types(type, class, description) VALUES ('furrier', 'poi', 'skins and pelts buyer');
INSERT INTO facet_types(type, class, description) VALUES ('gem_dealer', 'poi', 'gem buyer');
INSERT INTO facet_types(type, class, description) VALUES ('guild_task', 'poi', 'a guild-system task location (detail = <prof>:<object>)');
INSERT INTO facet_types(type, class, description) VALUES ('herb', 'resource', 'foraging source (detail = species)');
INSERT INTO facet_types(type, class, description) VALUES ('inn', 'poi', 'inn / lodging');
INSERT INTO facet_types(type, class, description) VALUES ('locker', 'poi', 'storage locker');
INSERT INTO facet_types(type, class, description) VALUES ('locksmith', 'poi', 'lockpick / box opener');
INSERT INTO facet_types(type, class, description) VALUES ('node', 'poi', 'resting node');
INSERT INTO facet_types(type, class, description) VALUES ('nomagic', 'property', 'magic is forbidden here');
INSERT INTO facet_types(type, class, description) VALUES ('pawnshop', 'poi', 'pawnshop / general buyer');
INSERT INTO facet_types(type, class, description) VALUES ('playershop', 'poi', 'player shop');
INSERT INTO facet_types(type, class, description) VALUES ('profession_guild', 'poi', 'a profession guild hall');
INSERT INTO facet_types(type, class, description) VALUES ('teleport_anchor', 'poi', 'teleport / portal anchor');
INSERT INTO facet_types(type, class, description) VALUES ('town_square', 'poi', 'central square / town center');
INSERT INTO facet_types(type, class, description) VALUES ('urchin', 'route', 'urchin travel network access');
INSERT INTO facet_types(type, class, description) VALUES ('voln', 'poi', 'Order of Voln master');
INSERT INTO facet_types(type, class, description) VALUES ('weaponshop', 'poi', 'weapon merchant');
