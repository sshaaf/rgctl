<?php
namespace App\Repository;

class UserRepository
{
    public function findByEmail(string $email): ?array
    {
        return ['email' => $email, 'id' => 1];
    }

    public function persist(array $user): void
    {
        // no-op persistence stub
    }
}
