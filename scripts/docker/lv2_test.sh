docker run -it --rm -v ~/Developer/rcompiler/:/root/compiler \
  -v ~/.cargo/config:/root/.cargo/config maxxing/compiler-dev \
  autotest -riscv -s lv1 /root/compiler
