# OSC

Shekere can receive **OSC (Open Sound Control)** messages from other applications like TidalCycles, Sonic Pi, or TouchDesigner.

## Connection Details

- **Default Port**: `2020`
- **Protocol**: UDP

## Handling OSC in Sketches

There are two ways to handle OSC data depending on your needs: **State** (latest value) or **Events** (triggers).

### 1. Persistent State (`osc`)
The `osc` object stores the latest data received at a specific address. This is ideal for continuous controls like faders or XY pads.

```javascript
export function update({ osc }) {
  // Get the latest value from /fader1 (assuming it's a number)
  const faderValue = osc['/fader1'] || 0;
  this.mesh.position.x = faderValue * 10;
}
```

### 2. Discrete Events (`oscEvents`)
The `oscEvents` array contains all OSC messages received **in the current frame**. This is ideal for triggers like drum beats or one-shot events.

```javascript
export function update({ oscEvents }) {
  oscEvents.forEach(event => {
    if (event.address === '/beat') {
      // Trigger a visual change on every beat
      this.triggerFlash();
    }
  });
}
```

## Special Support: TidalCycles

Shekere includes a built-in parser for **TidalCycles** (SuperDirt) messages sent to `/dirt/play`. 

Instead of an array of arguments, TidalCycles messages are converted into a friendly JavaScript object using its internal keys (e.g., `s`, `n`, `gain`, `cutoff`).

```javascript
export function update({ oscEvents }) {
  oscEvents.forEach(({ address, data }) => {
    if (address === '/dirt/play') {
      // 'data' is now an object like { s: "bd", gain: 1, ... }
      if (data.s === 'bd') {
        this.kickEffect();
      }
    }
  });
}
```

## Tips for OSC

1.  **UDP Traffic**: High-frequency OSC data can sometimes be dropped or arrive late over UDP. For critical sync, try to minimize the number of messages sent per frame.
2.  **Debugging**: Check the **Monitors** section in the Shekere Control Panel to inspect incoming OSC messages and their addresses in real-time.
3.  **External Port Mapping**: If you need to receive OSC on a different port, you may need to use a proxy tool, as the current port is fixed to 2020 in the application.
