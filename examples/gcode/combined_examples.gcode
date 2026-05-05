; Combined demonstration — use this to exercise all features in one file

; Start
G0X0Y0

; Move with feed and spindle speed
G1X100Y100F1500S1200 ; multiple params on one line

; Variables and macros
SET SPEED 1500
M3S1200

; Subprogram marker and call
O200
M98P200

; Extruder + parameter P
G1X100Y0E0.5P3 ; E axis and P parameter

; Comments
; Finished example
