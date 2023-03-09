docker run -it --rm -v ~/Developer/rcompiler:/root/compiler \
  -v ~/.cargo/config:/root/.cargo/config maxxing/compiler-dev \
  autotest -koopa -s lv4 /root/compiler
