set export

FLO_CARD := "1" # Video card, change this if you need

default: 
  just --list

dev-run preview="1":
  PREVIEW={preview} cargo run --features hot-reload

dev-lib:
  cargo watch -w lib -x 'build -p lib'

test-ffmpeg:
   ffmpeg -stream_loop -1 -loop 1 -i image.jpg -shortest -c:a aac -c:v libx264 -f flv -flvflags no_duration_filesize "rtmp://live.twitch.tv/app/$stream_key"
