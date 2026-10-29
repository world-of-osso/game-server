-- Shared by vehicle/accessory.rs (include_str!) and ground_files.py.
-- Spawned vehicles with TDB vehicle_template_accessory riders need seat poses.
-- LEFT JOIN preserves missing display/model metadata for explicit diagnostics.
SELECT DISTINCT m.CreatureDisplayID, t.VehicleId,
       d.ModelID, d.CreatureModelScale, md.FileDataID
FROM (SELECT DISTINCT entry FROM tdb_vehicle_template_accessory) a
JOIN content_creature_template t ON t.entry = a.entry
JOIN content_creature_template_model m ON m.CreatureID = a.entry
LEFT JOIN creature_display_info d ON d.ID = m.CreatureDisplayID
LEFT JOIN creature_model_data md ON md.ID = d.ModelID
WHERE t.VehicleId <> 0
  AND EXISTS (SELECT 1 FROM content_creature c WHERE c.id1 = a.entry)
ORDER BY t.VehicleId, m.CreatureDisplayID
