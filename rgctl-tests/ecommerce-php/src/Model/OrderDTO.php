<?php
namespace App\Model;

class OrderDTO
{
    public function __construct(private string $status) {}

    public function getStatus(): string
    {
        return $this->status;
    }
}
