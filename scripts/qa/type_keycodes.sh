#!/bin/zsh
# type_keycodes.sh <text> — type ASCII via key codes (bypasses IME)
set -e
typeset -A codes=(
  a 0 b 11 c 8 d 2 e 14 f 3 g 5 h 4 i 34 j 38 k 40 l 37 m 46
  n 45 o 31 p 35 q 12 r 15 s 1 t 17 u 32 v 9 w 13 x 7 y 16 z 6
  0 29 1 18 2 19 3 20 4 21 5 23 6 22 7 26 8 28 9 25
  ' ' 49 '-' 27 '=' 24 '[' 33 ']' 30 '\\' 42 ';' 41 "'" 39 ',' 43 '.' 47 '/' 44
)
text="$*"
osascript -e 'tell application "System Events" to tell process "Luma"
  if (count of windows) is 0 then return
  set focused of text field 1 of window 1 to true
  keystroke "a" using command down
  key code 51
end tell'
sleep 0.05
for (( i=1; i<=${#text}; i++ )); do
  ch="${text[$i]}"
  lc="${ch:l}"
  if [[ -n "${codes[$lc]}" ]]; then
    code="${codes[$lc]}"
    if [[ "$ch" != "$lc" ]]; then
      osascript -e "tell application \"System Events\" to tell process \"Luma\" to key code $code using {shift down}"
    else
      osascript -e "tell application \"System Events\" to tell process \"Luma\" to key code $code"
    fi
    sleep 0.02
  elif [[ "$ch" == " " ]]; then
    osascript -e 'tell application "System Events" to tell process "Luma" to key code 49'
    sleep 0.02
  fi
done
