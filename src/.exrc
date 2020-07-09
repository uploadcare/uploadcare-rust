if &cp | set nocp | endif
let s:cpo_save=&cpo
set cpo&vim
inoremap <silent> <C-Tab> =UltiSnips#ListSnippets()
map! <D-v> *
snoremap <silent>  "_c
xnoremap <silent> 	 :call UltiSnips#SaveLastVisualSelection()gvs
snoremap <silent> 	 :call UltiSnips#ExpandSnippet()
map  :cprevious
map  :cnext
map  <Plug>(ctrlp)
snoremap  "_c
nmap U viwUe
nnoremap <silent> \v :call go#lint#Vet(!g:go_jump_to_error)
nnoremap <silent> \l :call go#lint#Golint()
nnoremap \a :cclose
nnoremap \sv :source $MYVIMRC
nnoremap \ev :vsplit $MYVIMRC
nnoremap fo :grep <cword> ./**/*.go
vmap gx <Plug>NetrwBrowseXVis
nmap gx <Plug>NetrwBrowseX
nnoremap tn :NERDTreeToggle
nnoremap tt :TagbarTogglel
vnoremap <silent> <Plug>NetrwBrowseXVis :call netrw#BrowseXVis()
nnoremap <silent> <Plug>NetrwBrowseX :call netrw#BrowseX(netrw#GX(),netrw#CheckIfRemote(netrw#GX()))
snoremap <C-R> "_c
snoremap <silent> <C-H> "_c
snoremap <silent> <Del> "_c
snoremap <silent> <BS> "_c
snoremap <silent> <C-Tab> :call UltiSnips#ListSnippets()
map <C-P> <Plug>(ctrlp)
nnoremap <silent> <Plug>(ctrlp) :CtrlPMixed
nnoremap <Plug>(lsp-signature-help) :call lsp#ui#vim#signature_help#get_signature_help_under_cursor()
nnoremap <Plug>(lsp-previous-reference) :call lsp#ui#vim#references#jump(-1)
nnoremap <Plug>(lsp-next-reference) :call lsp#ui#vim#references#jump(+1)
nnoremap <Plug>(lsp-status) :echo lsp#get_server_status()
nnoremap <Plug>(lsp-peek-implementation) :call lsp#ui#vim#implementation(1)
nnoremap <Plug>(lsp-implementation) :call lsp#ui#vim#implementation(0)
xnoremap <Plug>(lsp-document-range-format) :<Home>silent <End>call lsp#ui#vim#document_range_format()
nnoremap <Plug>(lsp-document-range-format) :set opfunc=lsp#ui#vim#document_range_format_opfuncg@
vnoremap <Plug>(lsp-document-format) :<Home>silent <End>call lsp#ui#vim#document_range_format()
nnoremap <Plug>(lsp-document-format) :call lsp#ui#vim#document_format()
nnoremap <Plug>(lsp-workspace-symbol) :call lsp#ui#vim#workspace_symbol()
nnoremap <Plug>(lsp-peek-type-definition) :call lsp#ui#vim#type_definition(1)
nnoremap <Plug>(lsp-type-hierarchy) :call lsp#ui#vim#type_hierarchy()
nnoremap <Plug>(lsp-type-definition) :call lsp#ui#vim#type_definition(0)
nnoremap <Plug>(lsp-rename) :call lsp#ui#vim#rename()
nnoremap <Plug>(lsp-references) :call lsp#ui#vim#references()
nnoremap <Plug>(lsp-previous-diagnostic-nowrap) :call lsp#ui#vim#diagnostics#previous_diagnostic("--nowrap")
nnoremap <Plug>(lsp-previous-diagnostic) :call lsp#ui#vim#diagnostics#previous_diagnostic()
nnoremap <Plug>(lsp-next-diagnostic-nowrap) :call lsp#ui#vim#diagnostics#next_diagnostic("--nowrap")
nnoremap <Plug>(lsp-next-diagnostic) :call lsp#ui#vim#diagnostics#next_diagnostic()
nnoremap <Plug>(lsp-previous-warning-nowrap) :call lsp#ui#vim#diagnostics#previous_warning("--nowrap")
nnoremap <Plug>(lsp-previous-warning) :call lsp#ui#vim#diagnostics#previous_warning()
nnoremap <Plug>(lsp-next-warning-nowrap) :call lsp#ui#vim#diagnostics#next_warning("--nowrap")
nnoremap <Plug>(lsp-next-warning) :call lsp#ui#vim#diagnostics#next_warning()
nnoremap <Plug>(lsp-previous-error-nowrap) :call lsp#ui#vim#diagnostics#previous_error("--nowrap")
nnoremap <Plug>(lsp-previous-error) :call lsp#ui#vim#diagnostics#previous_error()
nnoremap <Plug>(lsp-next-error-nowrap) :call lsp#ui#vim#diagnostics#next_error("--nowrap")
nnoremap <Plug>(lsp-next-error) :call lsp#ui#vim#diagnostics#next_error()
nnoremap <Plug>(lsp-preview-focus) :call lsp#ui#vim#output#focuspreview()
nnoremap <Plug>(lsp-preview-close) :call lsp#ui#vim#output#closepreview()
nnoremap <Plug>(lsp-hover) :call lsp#ui#vim#hover#get_hover_under_cursor()
nnoremap <Plug>(lsp-document-diagnostics) :call lsp#ui#vim#diagnostics#document_diagnostics()
nnoremap <Plug>(lsp-document-symbol) :call lsp#ui#vim#document_symbol()
nnoremap <Plug>(lsp-peek-definition) :call lsp#ui#vim#definition(1)
nnoremap <Plug>(lsp-definition) :call lsp#ui#vim#definition(0)
nnoremap <Plug>(lsp-peek-declaration) :call lsp#ui#vim#declaration(1)
nnoremap <Plug>(lsp-declaration) :call lsp#ui#vim#declaration(0)
nnoremap <Plug>(lsp-code-action) :call lsp#ui#vim#code_action()
map <C-M> :cprevious
map <C-N> :cnext
xmap <BS> "-d
vmap <D-x> "*d
vmap <D-c> "*y
vmap <D-v> "-d"*P
nmap <D-v> "*P
inoremap <silent> 	 =UltiSnips#ExpandSnippet()
inoremap jk 
let &cpo=s:cpo_save
unlet s:cpo_save
set autowrite
set background=dark
set fileencodings=ucs-bom,utf-8,default,latin1
set helplang=en
set hidden
set lazyredraw
set path=./*
set ruler
set rulerformat=%l,%v
set runtimepath=~/.vim,~/.vim/plugged/nerdtree,~/.vim/plugged/async.vim,~/.vim/plugged/vim-lsp,~/.vim/plugged/splitjoin.vim,~/.vim/plugged/ctrlp.vim,~/.vim/plugged/ultisnips,~/.vim/plugged/tagbar,~/.vim/plugged/vim-clang-format,~/.vim/plugged/vim-delve,~/.vim/plugged/vim-go,~/.vim/plugged/rust.vim,~/.vim/plugged/vim-rust-syntax-ext,~/.vim/plugged/vader.vim,/usr/local/share/vim/vimfiles,/usr/local/share/vim/vim82,/usr/local/share/vim/vimfiles/after,~/.vim/plugged/ultisnips/after,~/.vim/plugged/rust.vim/after,~/.vim/plugged/vim-rust-syntax-ext/after,~/.vim/after
set smartindent
set softtabstop=8
set synmaxcol=200
set updatetime=10
set window=65
" vim: set ft=vim :
