Start:
- Optional on Windows: install msvc build tool
- install rustup
- install rust-toolchains
- <code>cargo install cargo-make</code>
- <code>cargo install clippy</code>
- Optional: <code>cargo install flamegraph</code>
- install kroki cli
- <code>docker pull docker.udayun.com/yuzutech/kroki:latest</code>
- <code>docker run -p8000:8000 yuzutech/kroki</code>
- install texlive and install missing packages (local or docker)
- <code>cargo make all</code>

编译论文:
<code>cargo make paper</code>
仅编译论文图片:
<code>cargo make fig</code>
编译库:
<code>cargo make release</code>
运行测试:
<code>cargo make test</code>
运行benchmark和指标输出:
<code>cargo make bench</code>
编译测试但不进行指标计算:
<code>cargo make all_build</code>
全部执行:
<code>cargo make all</code>

输出位置:
- 论文图片(原始): ./paper/assets/figures/original/
- 论文图片(最小化svg): ./paper/assets/figures/
- 论文: ./paper/builds/
- 库二进制文件(debug): ./target/debug/ 
- 库二进制文件(release): ./target/release/ 
- 测试结果和指标输出: ./target/criterion/
