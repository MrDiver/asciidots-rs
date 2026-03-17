let SessionLoad = 1
let s:so_save = &g:so | let s:siso_save = &g:siso | setg so=0 siso=0 | setl so=-1 siso=-1
let v:this_session=expand("<sfile>:p")
silent only
silent tabonly
cd ~/repositories/asciidots_playground/AsciidotsCompiler
if expand('%') == '' && !&modified && line('$') <= 1 && getline(1) == ''
  let s:wipebuf = bufnr('%')
endif
let s:shortmess_save = &shortmess
if &shortmess =~ 'A'
  set shortmess=aoOA
else
  set shortmess=aoO
endif
badd +279 asciidots-core/src/state_machine/states.rs
badd +69 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/parser/error.rs
badd +132 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/state_machine/machine.rs
badd +11 asciidots-core/src/state_machine/event.rs
badd +96 asciidots-core/src/state_machine/dot.rs
badd +371 asciidots-tui/src/lib.rs
badd +2 asciidots-tui/src/main.rs
badd +38 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/parser/token.rs
badd +50 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/tests/common/trace_harness.rs
badd +46 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-cli/src/lib.rs
badd +85 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/benches/interpreter_bench.rs
badd +95 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/interpreter/runtime.rs
badd +23 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/interpreter/field.rs
badd +1 ~/repositories/asciidots_playground/AsciidotsCompiler/samples/assertions.dots
badd +36 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/state_machine/error.rs
badd +7 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/interpreter/mod.rs
badd +11 asciidots-core/src/state_machine/mod.rs
badd +15 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/tests/dot_positions.rs
badd +1 asciidots-core/src/interpreter/operators.rs
badd +4 asciidots-core/Cargo.toml
badd +1 Cargo.toml
badd +1 Cargo.lock
badd +1 asciidots-cli/Cargo.toml
badd +41 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/presentation/models/output_list.rs
badd +47 asciidots-core/src/presentation/model_traits/list.rs
badd +1 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/presentation/models/dot_debug_table.rs
badd +39 asciidots-cli/src/main.rs
badd +2 samples/test_input.dots
badd +52 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/parser/lexer.rs
badd +124 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/presentation/controller.rs
badd +6 samples/transitions.dots
badd +57 ~/repositories/asciidots_playground/AsciidotsCompiler/asciidots-core/src/presentation/models/ascii_map.rs
badd +4 samples/conditional.dots
argglobal
%argdel
$argadd .
edit asciidots-core/src/state_machine/states.rs
argglobal
balt asciidots-core/src/state_machine/dot.rs
setlocal foldmethod=manual
setlocal foldexpr=0
setlocal foldmarker={{{,}}}
setlocal foldignore=#
setlocal foldlevel=0
setlocal foldminlines=1
setlocal foldnestmax=20
setlocal foldenable
silent! normal! zE
let &fdl = &fdl
let s:l = 279 - ((14 * winheight(0) + 17) / 35)
if s:l < 1 | let s:l = 1 | endif
keepjumps exe s:l
normal! zt
keepjumps 279
normal! 084|
tabnext 1
if exists('s:wipebuf') && len(win_findbuf(s:wipebuf)) == 0 && getbufvar(s:wipebuf, '&buftype') isnot# 'terminal'
  silent exe 'bwipe ' . s:wipebuf
endif
unlet! s:wipebuf
set winheight=1 winwidth=20
let &shortmess = s:shortmess_save
let s:sx = expand("<sfile>:p:r")."x.vim"
if filereadable(s:sx)
  exe "source " . fnameescape(s:sx)
endif
let &g:so = s:so_save | let &g:siso = s:siso_save
set hlsearch
nohlsearch
doautoall SessionLoadPost
unlet SessionLoad
" vim: set ft=vim :
