<?php
namespace App\Service;

use App\Repository\UserRepository;
use App\Model\OrderDTO;
use App\Traits\Timestampable;

class AuthService
{
    use Timestampable;

    public const VERSION = '1.0';

    public function __construct(private UserRepository $repository) {}

    public function login(string $email): ?array
    {
        $user = $this->repository->findByEmail($email);
        if ($user === null) {
            return null;
        }
        $this->repository->persist($user);
        return $user;
    }

    public function unsafeQuery(): void
    {
        $id = $_GET['id'];
        $query = "SELECT * FROM users WHERE id = " . $id;
        mysqli_query($GLOBALS['conn'], $query);
    }

    public function processOrder(OrderDTO $order): void
    {
        $order->status = 'PROCESSED';
    }
}
