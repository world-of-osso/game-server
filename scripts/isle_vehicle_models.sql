-- TC IoC NPC_DEMOLISHER: source VehicleId and display/model bind poses.
-- Shared by vehicle/isle.rs and ground_files.py; no mount or faction overrides.
SELECT DISTINCT m.CreatureDisplayID, t.VehicleId,
       d.ModelID, d.CreatureModelScale, md.FileDataID
FROM content_creature_template t
JOIN content_creature_template_model m ON m.CreatureID = t.entry
LEFT JOIN creature_display_info d ON d.ID = m.CreatureDisplayID
LEFT JOIN creature_model_data md ON md.ID = d.ModelID
WHERE t.entry = 34775 AND t.VehicleId <> 0
  AND EXISTS (SELECT 1 FROM content_creature c WHERE c.id1 = t.entry AND c.map = 628)
ORDER BY t.VehicleId, m.CreatureDisplayID
