<?php
namespace App\Fixtures;

class PropertyHookSample
{
    public string $label {
        get => strtoupper($this->label);
        set => $this->label = $value;
    }
}
