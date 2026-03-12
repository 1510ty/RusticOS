from PIL import Image
import sys

def convert_to_rust_glyph(image_path, char_name):
    # 画像読み込み
    img = Image.open(image_path).convert('L')
    img = img.resize((24, 24)) # 24x24にリサイズ
    
    threshold = 128
    rows = []
    for y in range(24):
        row_val = 0
        for x in range(24):
            # 輝度がしきい値より低ければ（黒ければ）ビットを立てる
            # 31bit目から順に左詰めで格納
            if img.getpixel((x, y)) < threshold:
                row_val |= (1 << (31 - x))
        rows.append(f"0x{row_val:08x}")

    # Unicodeコードポイントを取得
    unicode_hex = f"{ord(char_name):04X}"
    
    print(f"// Character: '{char_name}' (U+{unicode_hex})")
    # 配列の型を [u32; 24] に変更
    print(f"pub const GLYPH_{unicode_hex}: [u32; 24] = [")
    # 24行分を4つずつ出力 (24 / 4 = 6セット)
    for i in range(0, 24, 4):
        print(f"    {rows[i]}, {rows[i+1]}, {rows[i+2]}, {rows[i+3]},")
    print("];\n")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python convert.py <image_file> <char>")
    else:
        convert_to_rust_glyph(sys.argv[1], sys.argv[2])