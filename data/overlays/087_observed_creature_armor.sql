-- Creature armor as the game rolled it, where lich-5 says otherwise.
--
-- A roll prints its AvD, and the AvD is fixed by the weapon and the armor
-- sub-group it lands on (`attack_avd`). So a creature's sub-group can be read
-- back off the rolls against it -- a measurement, where lich-5's value is a
-- transcription. When the two disagree the measurement wins, and it lives
-- here so the next `codex creatures` harvest does not quietly revert it.
--
-- Ithzir herald: lich-5 lists sub-group 9. 24 closed-fist swings in the
-- corpus (GST, Norhaak) all rolled AvD +6, which is closed fist against
-- sub-group 11; against 9 it would be +10. The same swings at the Ithzir seer
-- and adept (+19, sub-group 6) and the war griffin (+4, sub-group 12) agree
-- with lich, so the method is sound and the herald is the outlier.
UPDATE creature_defenses
   SET asg = 11
 WHERE creature = 'Ithzir herald';
