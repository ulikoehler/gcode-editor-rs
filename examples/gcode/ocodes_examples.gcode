; O-code examples
; O-codes (subprogram or program numbers) and simple control flow examples

; Subprogram O100 demonstrates a small loop using a WHILE construct
O100 (Subprogram start)
#100=0 ; initialize loop counter `#100` to 0 (used by the following WHILE loop)
WHILE[#100 LT 3] DO
  G1X[#100*10]Y0 ; move based on loop counter
  #100=#100+1
ENDWHILE
M99 ; return from subprogram

; Call the subprogram using M98 (common on many controllers)
M98P100 L2 ; call O100 twice (L is loop count)

; Another program marker
O200
G0X0Y0
M30 ; program end
