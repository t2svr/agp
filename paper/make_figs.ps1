$files = Get-ChildItem  -Path ./paper/src/figures/ -Recurse
foreach ($f in $files) {
    $ext = [io.path]::GetExtension($f.FullName).TrimStart('.')
    $savePath = './paper/assets/figures/' + [io.path]::ChangeExtension($f.Name, ".svg")
    kroki convert $f.FullName --type $ext --format svg --out-file $savePath
} 