import { invoke } from "@tauri-apps/api/core";

const calculateButton = document.querySelector<HTMLButtonElement>('#calculate')!;
const resultDiv = document.querySelector<HTMLDivElement>('#result')!;

calculateButton.addEventListener('click', async () => {
  const startTime = performance.now();
  try {
    const result = await invoke('risk_normalization_command') as {
      safe_f_mean: number;
      safe_f_stdev: number;
      car25_mean: number;
      car25_stdev: number;
    };
    const endTime = performance.now();
    const elapsedTime = ((endTime - startTime) / 1000).toFixed(2);

    resultDiv.innerHTML = `
      <p>CAR25 Mean: ${result.car25_mean.toFixed(5)}%</p>
      <p>CAR25 StdDev: ${result.car25_stdev.toFixed(5)}</p>
      <p>Safe-f Mean: ${result.safe_f_mean.toFixed(5)}</p>
      <p>Safe-f StdDev: ${result.safe_f_stdev.toFixed(5)}</p>
      <p>Elapsed Time: ${elapsedTime} seconds</p>
    `;
  } catch (error) {
    resultDiv.textContent = `Error: ${error}`;
  }
});