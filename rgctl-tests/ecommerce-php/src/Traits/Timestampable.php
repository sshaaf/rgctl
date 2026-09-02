<?php
namespace App\Traits;

trait Timestampable
{
    public function touch(): void
    {
        $this->updatedAt = time();
    }
}
