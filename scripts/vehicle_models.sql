-- Shared by vehicle.rs (include_str!) and ground_files.py.
-- Only enter/exit, non-control seats need server-side passenger bind poses.
-- LEFT JOIN preserves missing display/model metadata for explicit diagnostics.
SELECT DISTINCT x.CreatureDisplayInfoID, c.VehicleId,
       d.ModelID, d.CreatureModelScale, md.FileDataID
FROM mount m
JOIN mount_x_display x ON x.MountID = m.ID
JOIN spell_effect e ON e.SpellID = m.SourceSpellID
JOIN creature_template c ON c.entry = e.EffectMiscValue_0
LEFT JOIN creature_display_info d ON d.ID = x.CreatureDisplayInfoID
LEFT JOIN creature_model_data md ON md.ID = d.ModelID
WHERE e.EffectAura = 78 AND e.DifficultyID = 0 AND c.VehicleId <> 0
  AND EXISTS (
      SELECT 1 FROM vehicle v
      JOIN vehicle_seat s ON s.ID IN (
          v.SeatID_0, v.SeatID_1, v.SeatID_2, v.SeatID_3,
          v.SeatID_4, v.SeatID_5, v.SeatID_6, v.SeatID_7
      )
      WHERE v.ID = c.VehicleId AND s.ID <> 0
        AND (s.Flags & 33554432) <> 0 -- CAN_ENTER_OR_EXIT
        AND (s.Flags & 2048) = 0     -- CAN_CONTROL
  )
ORDER BY c.VehicleId, x.CreatureDisplayInfoID
