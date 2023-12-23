# lwa_unity_unpack

Simple CLI tool for unpacking the unitypackages. Also allows auto convert of the FBX files to GLTF during unpacking.

```bash
Program for unpacking unitypackages files

Usage: lwa_unity_unpack.exe [OPTIONS] --input <INPUT> --output <OUTPUT>

Options:
  -i, --input <INPUT>              .unitypackage file to extract
  -o, --output <OUTPUT>            target directory
  -f, --fbx-to-gltf <FBX_TO_GLTF>  optional- path to the tool that will auto convert fbx files to gltf during unpacking
  -h, --help                       Print help
  -V, --version                    Print version
  ```


`lwa_unity_unpack -i "C:\\PROJECTS\\lwa_unity_unpack\\POLYGON_Snow_Kit_Unity_2020_3_v1_4.unitypackage" -o "output" -f "C:\\tools\\FBX2glTF.exe"`
