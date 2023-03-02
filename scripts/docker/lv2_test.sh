docker run -it --rm -v ~/Developer/rcompiler/:/root/compiler maxxing/compiler-dev \
  autotest -riscv -s lv1 /root/compiler
