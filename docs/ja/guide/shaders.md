# シェーダー (TSL & WebGPU)

Shekereは、高性能なプロシージャルグラフィックスやエフェクトを実現するために、**Three.js Shading Language (TSL)** と **WebGPU** をサポートしています。

TSLは、「JavaScriptファースト」な統一されたシェーダー開発環境を提供します。

## TSLとは？

TSL (Three.js Shading Language) は、JavaScriptやTypeScriptのコードだけで、ノードベースのシェーダーを構築できる新しい仕組みです。生のGLSL文字列を書く代わりに、JavaScriptの関数を使って数学的な操作やグラフィックスのロジックを組み立てます。

ここで構築されたJavaScriptのノード構造は、実行時にThree.jsによって、高度に最適化されたWGSL (WebGPU Shading Language) へと自動的にコンパイルされます。

### なぜTSLなのか？

1. **言語の統一**: CPU側のロジック（JavaScript）とGPU側のシェーダーロジックを、全く同じ言語で書くことができます。
2. **再利用性**: シェーダーのパーツは単なるJavaScriptの関数です。自由にインポート・エクスポートし、無限に組み合わせることができます。
3. **WebGPUネイティブ**: TSLはモダンなWebGPUパイプラインのためにゼロから設計されており、コンピュートシェーダーの利用や圧倒的なパフォーマンスの向上が見込めます。

## ShekereでのTSLの利用

Shekereのスケッチファイル内では、グローバルオブジェクト `TSL` が自動的に利用可能になっています。インポート文を書く必要はありません。

```javascript
export function setup(scene) {
    // TSLの基本的な使い方
    const myColorNode = TSL.vec3(1.0, 0.0, 0.0); // 純粋な赤色のノード
    
    const material = new THREE.MeshBasicNodeMaterial();
    material.colorNode = myColorNode;

    const mesh = new THREE.Mesh(new THREE.BoxGeometry(), material);
    scene.add(mesh);
}
```

## カスタム関数 (Fn)

複雑な計算式になると、`.mul()` や `.add()` のようなメソッドチェーンは非常に読みにくくなります。TSLには `Fn()` という強力なラッパーが用意されており、これを使うとJavaScriptの構文で書いたロジックをそのままシェーダーコードに変換してくれます！

```javascript
const calculateGlow = TSL.Fn(() => {
    // TSL.uv() は現在のピクセルのUV座標を取得します
    const d = TSL.distance(TSL.uv(), TSL.vec2(0.5));
    
    // smoothstep で柔らかい円形のグラデーションを作ります
    let strength = TSL.smoothstep(0.5, 0.2, d);
    
    return strength;
});

const material = new THREE.MeshBasicNodeMaterial({ transparent: true });
material.opacityNode = calculateGlow();
material.colorNode = TSL.vec3(0.0, 1.0, 1.0); // シアン
```

## パーティクルシステムと InstancedMesh

**WebGPUにおける重要な注意点:** 標準の `THREE.Points` や `PointsMaterial` は、WebGPU環境では非常に強い制限を受けます。WebGPUの仕様上、**ポイントのサイズは常に「1ピクセル」に固定されます。** つまり、標準のPointsでは大きく輝くパーティクルを作ることはできません。

サイズ変更可能なパーティクルシステムを作成するには、**必ず `THREE.InstancedMesh` を使用し、** TSLの頂点ビルボード計算（常にカメラの方向を向かせる計算）と組み合わせる必要があります。

### 例: InstancedMesh を使ったパーティクル

`THREE.Points` の代わりに、小さな `PlaneGeometry` を数千個インスタンス化します。TSLの `vertexNode` を上書きすることで、すべての板ポリゴンが常にカメラを向くように（ビルボード化）設定します。

```javascript
export function setup(scene) {
    const COUNT = 1000;
    const geometry = new THREE.PlaneGeometry(1, 1);
    
    // 各インスタンスごとのカスタム属性（位置情報）を渡す
    const positions = new Float32Array(COUNT * 3);
    for(let i = 0; i < COUNT * 3; i++) positions[i] = (Math.random() - 0.5) * 10;
    geometry.setAttribute('instanceOffset', new THREE.InstancedBufferAttribute(positions, 3));

    const material = new THREE.MeshBasicNodeMaterial();

    // 1. カスタム属性を読み込む
    const instanceOffset = TSL.attribute('instanceOffset', 'vec3');
    
    // 2. ビルボード計算（常にカメラの正面を向く）
    const viewOffset = TSL.cameraViewMatrix.mul(TSL.modelWorldMatrix).mul(TSL.vec4(instanceOffset, 1.0));
    const finalViewPos = viewOffset.add(TSL.vec4(TSL.positionLocal.xy, 0.0, 0.0));
    
    // 3. デフォルトの頂点変換処理を上書き
    material.vertexNode = TSL.cameraProjectionMatrix.mul(finalViewPos);

    const instancedMesh = new THREE.InstancedMesh(geometry, material, COUNT);
    // シェーダー側で強制的に位置をずらしているため、カリング（画面外の描画省略）を無効化する
    instancedMesh.frustumCulled = false; 
    
    scene.add(instancedMesh);
}
```

より高度な実装（MIDI入力に反応するパーティクルなど）については、リポジトリ内の `shader_stars.js` サンプルコードを参考にしてください！
