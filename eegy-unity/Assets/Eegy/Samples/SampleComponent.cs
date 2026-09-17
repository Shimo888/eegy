using UnityEngine;

namespace Eegy.Samples
{
    public class SampleComponent : MonoBehaviour
    {
        void Start()
        {
            Execute();
        }

        private void Execute()
        {
            var result = Native.NativeMethods.sample_func_add(1, 2);
            Debug.Log(result);
        }
    }
}
