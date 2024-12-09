#// Copyright 2024 Junshuang Hu
$files = Get-ChildItem  -Path ./paper/src/figures/ -Recurse
$saveDir = './paper/assets/figures/original'
$optOutDir = './paper/assets/figures'
$svgCfgPath = './svgo.config.mjs'
foreach ($f in $files) {
    $ext = [io.path]::GetExtension($f.FullName).TrimStart('.')
    $saveName = [io.path]::ChangeExtension($f.Name, ".svg")
    $saveFullPath =  $saveDir + '/' + $saveName
    kroki convert $f.FullName --type $ext --format svg --out-file $saveFullPath --config ./kroki.yml
} 
svgo -rf $saveDir -o $optOutDir --config $svgCfgPath