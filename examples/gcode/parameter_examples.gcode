; Parameter highlighting examples
; F=feed, S=spindle or speed, T=tool, P=parameter (special color)

G1X50Y50F1500S500
M3S100 ; spindle on with speed
T1 ; tool change
P10 ; standalone P parameter
G1X0Y0F1e3 ; exponential feed number
